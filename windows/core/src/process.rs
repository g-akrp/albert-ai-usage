use std::{
    collections::{BTreeMap, HashMap},
    ffi::c_void,
    fs::File,
    io::Write,
    os::windows::io::{AsRawHandle, FromRawHandle},
    path::Path,
    sync::{Mutex, OnceLock},
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::*,
        Security::SECURITY_ATTRIBUTES,
        System::{JobObjects::*, Pipes::CreatePipe, Threading::*},
    },
};

static JOBS: OnceLock<Mutex<HashMap<usize, (usize, usize)>>> = OnceLock::new();
pub fn cancel_jobs(owner: usize) {
    let guard = JOBS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for (job, token) in guard.values() {
        if *token == owner {
            unsafe {
                let _ = TerminateJobObject(HANDLE(*job as *mut c_void), 1);
            }
        }
    }
}
pub struct Handle(pub HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}
fn error(operation: &str, e: windows::core::Error) -> String {
    format!("{operation} failed ({:08x})", e.code().0 as u32)
}
fn pipe() -> Result<(Handle, Handle), String> {
    let mut read = HANDLE::default();
    let mut write = HANDLE::default();
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        bInheritHandle: true.into(),
        ..Default::default()
    };
    unsafe {
        CreatePipe(&mut read, &mut write, Some(&attributes), 0).map_err(|e| error("pipe", e))?;
    }
    Ok((Handle(read), Handle(write)))
}
pub struct Child {
    pub process: Handle,
    pub job: Handle,
    pub stdin: Option<File>,
    id: usize,
}
impl Drop for Child {
    fn drop(&mut self) {
        let mut jobs = JOBS
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        jobs.remove(&self.id);
        unsafe {
            let _ = TerminateJobObject(self.job.0, 0);
        }
        self.stdin.take();
    }
}
impl Child {
    pub fn terminate(&self) {
        unsafe {
            let _ = TerminateJobObject(self.job.0, 1);
        }
    }
    pub fn spawn(
        exe: &Path,
        args: &[String],
        env: &BTreeMap<String, String>,
        cwd: &Path,
        owner: usize,
    ) -> Result<(Self, File), String> {
        let (input, write) = pipe()?;
        let (read, output) = pipe()?;
        unsafe {
            SetHandleInformation(read.0, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0))
                .map_err(|e| error("pipe inheritance", e))?;
            SetHandleInformation(write.0, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0))
                .map_err(|e| error("pipe inheritance", e))?;
        }
        let nul = std::fs::OpenOptions::new()
            .write(true)
            .open("NUL")
            .map_err(|_| "could not open null output")?;
        let stderr = HANDLE(nul.as_raw_handle());
        unsafe {
            SetHandleInformation(stderr, HANDLE_FLAG_INHERIT.0, HANDLE_FLAG_INHERIT)
                .map_err(|e| error("null inheritance", e))?;
        }
        let job =
            Handle(unsafe { CreateJobObjectW(None, PCWSTR::null()).map_err(|e| error("job", e))? });
        let mut limit = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limit.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limit as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limit) as u32,
            )
            .map_err(|e| error("job limits", e))?;
        }
        let mut size = 0;
        unsafe {
            let _ = InitializeProcThreadAttributeList(None, 1, None, &mut size);
        }
        let mut storage = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
        unsafe {
            InitializeProcThreadAttributeList(Some(list), 1, None, &mut size)
                .map_err(|e| error("handle list", e))?;
        }
        struct Attributes(LPPROC_THREAD_ATTRIBUTE_LIST);
        impl Drop for Attributes {
            fn drop(&mut self) {
                unsafe {
                    DeleteProcThreadAttributeList(self.0);
                }
            }
        }
        let _attributes = Attributes(list);
        let handles = [input.0, output.0, stderr];
        unsafe {
            UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                Some(handles.as_ptr().cast()),
                std::mem::size_of_val(&handles),
                None,
                None,
            )
            .map_err(|e| error("inherited handles", e))?;
        }
        let mut startup = STARTUPINFOEXW {
            lpAttributeList: list,
            ..Default::default()
        };
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = input.0;
        startup.StartupInfo.hStdOutput = output.0;
        startup.StartupInfo.hStdError = stderr;
        let exe_text = exe.to_string_lossy().to_string();
        let shim = exe
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("cmd") || s.eq_ignore_ascii_case("bat"));
        let (application, command) = if shim {
            // Prevent cmd expansion instead of interpolating untrusted shell metacharacters.
            if args
                .iter()
                .any(|a| a.chars().any(|c| ['\r', '\n', '%', '"'].contains(&c)))
            {
                return Err("unsupported quote, percent or newline in batch arguments".into());
            }
            let application = env
                .get("ComSpec")
                .cloned()
                .unwrap_or_else(|| "C:\\Windows\\System32\\cmd.exe".into());
            let inner = std::iter::once(crate::exe_lookup::quote(&exe_text))
                .chain(args.iter().map(|a| crate::exe_lookup::quote(a)))
                .collect::<Vec<_>>()
                .join(" ");
            let command = format!(
                "{} /d /v:off /s /c \"{}\"",
                crate::exe_lookup::quote(&application),
                inner
            );
            (application, command)
        } else {
            let command = std::iter::once(crate::exe_lookup::quote(&exe_text))
                .chain(args.iter().map(|a| crate::exe_lookup::quote(a)))
                .collect::<Vec<_>>()
                .join(" ");
            (exe_text, command)
        };
        let application: Vec<u16> = application.encode_utf16().chain(Some(0)).collect();
        let mut command: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
        let cwd: Vec<u16> = cwd.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut environment = Vec::<u16>::new();
        let mut variables: Vec<_> = env.iter().collect();
        variables.sort_by_key(|(k, _)| k.to_uppercase());
        for (k, v) in variables {
            environment.extend(format!("{k}={v}").encode_utf16());
            environment.push(0);
        }
        environment.push(0);
        let mut info = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcessW(
                PCWSTR(application.as_ptr()),
                Some(PWSTR(command.as_mut_ptr())),
                None,
                None,
                true,
                CREATE_NO_WINDOW
                    | CREATE_SUSPENDED
                    | CREATE_UNICODE_ENVIRONMENT
                    | EXTENDED_STARTUPINFO_PRESENT,
                Some(environment.as_ptr().cast()),
                PCWSTR(cwd.as_ptr()),
                &startup.StartupInfo,
                &mut info,
            )
            .map_err(|e| error("start CLI", e))?;
        }
        let process = Handle(info.hProcess);
        let thread = Handle(info.hThread);
        unsafe {
            if let Err(e) = AssignProcessToJobObject(job.0, process.0) {
                let _ = TerminateProcess(process.0, 1);
                return Err(error("job assignment", e));
            }
        }
        let id = info.dwProcessId as usize;
        JOBS.get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, (job.0 .0 as usize, owner));
        let stdin = unsafe { File::from_raw_handle(write.0 .0) };
        std::mem::forget(write);
        let stdout = unsafe { File::from_raw_handle(read.0 .0) };
        std::mem::forget(read);
        let child = Self {
            process,
            job,
            stdin: Some(stdin),
            id,
        };
        unsafe {
            if ResumeThread(thread.0) == u32::MAX {
                return Err("could not resume CLI".into());
            }
        }
        drop(input);
        drop(output);
        drop(nul);
        Ok((child, stdout))
    }
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.stdin
            .as_mut()
            .ok_or("CLI stdin closed")?
            .write_all(bytes)
            .map_err(|_| "could not write CLI request".into())
    }
    pub fn exit_status(&self, timeout: u32) -> Result<u32, String> {
        unsafe {
            if WaitForSingleObject(self.process.0, timeout) != WAIT_OBJECT_0 {
                return Err("CLI timed out".into());
            }
            let mut code = 0;
            GetExitCodeProcess(self.process.0, &mut code).map_err(|e| error("exit status", e))?;
            Ok(code)
        }
    }
}
use std::os::windows::ffi::OsStrExt;
