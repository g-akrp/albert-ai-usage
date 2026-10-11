use crate::{
    card_model::Run,
    child_env,
    config::{ProviderConfig, Source},
    exe_lookup, json, process,
    report::Report,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
        process::cancel_jobs(self.key());
    }
    pub fn cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    fn key(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}
enum Output {
    Bytes(Vec<u8>),
    End,
    Error(String),
}
static NEXT: AtomicU64 = AtomicU64::new(0);
pub fn capture(source: &Source, cancel: &Cancellation) -> Result<Value, String> {
    if cancel.cancelled() {
        return Err("cancelled".into());
    }
    let env = child_env::environment(&source.env);
    let exe = exe_lookup::find(&source.executable, &env)
        .ok_or_else(|| format!("{} not found on PATH", source.executable))?;
    let directory = std::env::temp_dir().join(format!(
        "ai-usage-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).map_err(|_| "could not create temporary working directory")?;
    let result = run_capture(source, cancel, &exe, &env, &directory);
    // Remove only the unique directory this capture created; no user path is used here.
    if let (Ok(target), Ok(root)) = (
        directory.canonicalize(),
        std::env::temp_dir().canonicalize(),
    ) {
        if target.parent() == Some(root.as_path())
            && target.file_name().is_some_and(|n| {
                n.to_string_lossy()
                    .starts_with(&format!("ai-usage-{}-", std::process::id()))
            })
        {
            let _ = std::fs::remove_dir_all(&target);
        }
    }
    result
}
fn run_capture(
    source: &Source,
    cancel: &Cancellation,
    exe: &std::path::Path,
    env: &BTreeMap<String, String>,
    directory: &std::path::Path,
) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(source.timeout);
    let (mut child, mut stdout) =
        process::Child::spawn(exe, &source.args, env, directory, cancel.key())?;
    if cancel.cancelled() {
        return Err("cancelled".into());
    }
    let (tx, rx) = mpsc::sync_channel(8);
    let cap = if source.kind == "stdio" {
        source.max_total
    } else {
        source.max_output
    };
    let reader = std::thread::spawn(move || {
        let mut total = 0;
        let mut buffer = [0u8; 4096];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => {
                    let _ = tx.send(Output::End);
                    break;
                }
                Ok(n) => {
                    total += n;
                    if total > cap {
                        let _ =
                            tx.send(Output::Error(format!("CLI printed more than {cap} bytes")));
                        break;
                    }
                    if tx.send(Output::Bytes(buffer[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(Output::Error("could not read CLI output".into()));
                    break;
                }
            }
        }
    });
    let get = || -> Result<Output, String> {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err("CLI timed out".into());
        }
        if cancel.cancelled() {
            return Err("cancelled".into());
        }
        rx.recv_timeout(left).map_err(|_| "CLI timed out".into())
    };
    let result = (|| {
        if source.kind == "command" {
            child.stdin.take();
            let mut data = Vec::new();
            loop {
                match get()? {
                    Output::Bytes(b) => data.extend(b),
                    Output::End => break,
                    Output::Error(e) => return Err(e),
                }
            }
            let exit = child.exit_status(
                deadline
                    .saturating_duration_since(Instant::now())
                    .as_millis()
                    .min(u32::MAX as u128) as u32,
            )?;
            if exit != 0 {
                return Err(format!("CLI exited with status {exit}"));
            }
            let value = json::parse(&data).map_err(|_| "CLI output was not valid JSON")?;
            check(&source.raw["expect"], &value)?;
            return Ok(value);
        }
        let mut pending = Vec::new();
        let mut captured = BTreeMap::new();
        for step in source.raw["steps"]
            .as_array()
            .ok_or("missing stdio steps")?
        {
            if let Some(message) = step.get("write") {
                let mut bytes = serde_json::to_vec(message).map_err(|_| "invalid request")?;
                bytes.push(b'\n');
                write_request(&mut child, bytes, deadline)?;
                continue;
            }
            let expected = &step["await"];
            loop {
                let line = loop {
                    if let Some(end) = pending.iter().position(|b| *b == b'\n') {
                        let line: Vec<u8> = pending.drain(..=end).collect();
                        break line;
                    }
                    if pending.len() > source.max_line {
                        return Err(format!("CLI line exceeded {} bytes", source.max_line));
                    }
                    match get()? {
                        Output::Bytes(b) => {
                            pending.extend(b);
                            if pending
                                .iter()
                                .position(|b| *b == b'\n')
                                .unwrap_or(pending.len())
                                > source.max_line
                            {
                                return Err(format!("CLI line exceeded {} bytes", source.max_line));
                            }
                        }
                        Output::End => return Err("CLI closed before usage response".into()),
                        Output::Error(e) => return Err(e),
                    }
                };
                let Ok(value) = json::parse(&line) else {
                    continue;
                };
                if !json::matches(&expected["match"], &value) {
                    continue;
                }
                check(expected, &value)?;
                if let Some(name) = expected["capture"].as_str() {
                    captured.insert(name.to_string(), value);
                }
                break;
            }
        }
        child.stdin.take();
        captured
            .remove(
                source.raw["output"]
                    .as_str()
                    .ok_or("missing capture name")?,
            )
            .ok_or_else(|| "no captured usage response".into())
    })();
    drop(child);
    drop(rx);
    let _ = reader.join();
    if cancel.cancelled() {
        Err("cancelled".into())
    } else {
        result
    }
}

fn write_request(
    child: &mut process::Child,
    bytes: Vec<u8>,
    deadline: Instant,
) -> Result<(), String> {
    let mut pipe = child.stdin.take().ok_or("CLI stdin closed")?;
    let (tx, rx) = mpsc::sync_channel(1);
    let writer = std::thread::spawn(move || {
        let result = pipe
            .write_all(&bytes)
            .map_err(|_| "could not write CLI request".to_string());
        let _ = tx.send((pipe, result));
    });
    let output = rx.recv_timeout(deadline.saturating_duration_since(Instant::now()));
    if output.is_err() {
        child.terminate();
    }
    drop(rx);
    let _ = writer.join();
    match output {
        Ok((pipe, result)) => {
            child.stdin = Some(pipe);
            result
        }
        Err(_) => Err("CLI timed out writing request".into()),
    }
}
fn check(expect: &Value, value: &Value) -> Result<(), String> {
    if !json::matches(&expect["match"], value) {
        return Err("CLI response did not match".into());
    }
    if let Some(p) = expect["error"].as_str() {
        if let Some(e) = value.pointer(p).filter(|v| !v.is_null()) {
            return Err(if e["code"].as_i64() == Some(-32601) {
                "Update the CLI to see usage"
            } else {
                "CLI reported an error"
            }
            .into());
        }
    }
    if let Some(p) = expect["require"].as_str() {
        if value.pointer(p).is_none_or(Value::is_null) {
            return Err("CLI response missing required usage data".into());
        }
    }
    Ok(())
}
pub fn run(config: &ProviderConfig, cancel: &Cancellation) -> Vec<Run> {
    let accounts = if let Some(a) = &config.accounts {
        let result = Source::parse(&a["source"]).and_then(|s| capture(&s, cancel));
        match result {
            Ok(value) => {
                let mut ids = Vec::new();
                if let Some(rows) = a["each"]
                    .as_str()
                    .and_then(|p| value.pointer(p))
                    .and_then(Value::as_array)
                {
                    for row in rows.iter().take(32) {
                        if json::matches(&a["match"], row) {
                            if let Some(id) =
                                a["id"].as_str().and_then(|p| json::string(row.pointer(p)))
                            {
                                if id.len() <= 256 && !ids.contains(&id) {
                                    ids.push(id);
                                }
                            }
                        }
                    }
                }
                if ids.is_empty() {
                    let mut r = Run::loading(config);
                    r.result = Some(Err("no accounts found".into()));
                    return vec![r];
                }
                ids.into_iter().map(Some).collect::<Vec<_>>()
            }
            Err(e) => {
                let mut r = Run::loading(config);
                r.result = Some(Err(format!("could not list accounts: {e}")));
                return vec![r];
            }
        }
    } else {
        vec![None]
    };
    let multiple = accounts.len() > 1;
    accounts
        .into_iter()
        .enumerate()
        .map(|(i, account)| {
            let mut r = Run::loading(config);
            r.account = account.clone();
            if let Some(id) = &account {
                r.id = format!("{}:{id}", config.id);
                if multiple {
                    r.label = format!("GH{}", (i + 1).min(9));
                }
            }
            let source = account
                .as_deref()
                .map(|a| config.source.account(a))
                .unwrap_or_else(|| config.source.clone());
            r.result =
                Some(capture(&source, cancel).and_then(|v| Report::map(&config.mapping, &v)));
            r
        })
        .collect()
}
