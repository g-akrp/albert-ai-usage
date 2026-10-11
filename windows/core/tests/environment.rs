use ai_usage_core::child_env::environment;
use std::collections::BTreeMap;

#[test]
fn environment_overrides_are_case_insensitive_and_tokens_are_reserved() {
    let e = environment(&BTreeMap::from([
        ("path".into(), "C:\\fixture".into()),
        ("gh_token".into(), "fixture-token".into()),
    ]));
    assert_eq!(e.get("PATH").map(String::as_str), Some("C:\\fixture"));
    assert!(!e
        .keys()
        .any(|k| k.eq_ignore_ascii_case("GH_TOKEN") || k.eq_ignore_ascii_case("GITHUB_TOKEN")));
    let keys = e
        .keys()
        .map(|k| k.to_uppercase())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(keys.len(), e.len());
}
#[test]
fn windows_lookup_prefers_cmd_over_extensionless_posix_shim() {
    let folder = std::env::temp_dir().join(format!("ai-usage-lookup-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("codex"), "#!/bin/sh\n").unwrap();
    std::fs::write(folder.join("codex.cmd"), "@echo off\n").unwrap();
    let env = std::collections::BTreeMap::from([
        ("PATH".into(), folder.to_string_lossy().into_owned()),
        ("PATHEXT".into(), ".EXE;.CMD".into()),
    ]);
    assert_eq!(
        ai_usage_core::exe_lookup::find("codex", &env)
            .unwrap()
            .canonicalize()
            .unwrap(),
        folder.join("codex.cmd").canonicalize().unwrap()
    );
    std::fs::remove_file(folder.join("codex")).unwrap();
    std::fs::remove_file(folder.join("codex.cmd")).unwrap();
    std::fs::remove_dir(folder).unwrap();
}
