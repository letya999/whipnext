//! OS-level inject of the user prompt `next` + Enter into a TUI process.

use crate::session::Target;

pub fn live_inject(target: &Target) -> Result<(), String> {
    live_inject_with(target, crate::inject::NEXT_PROMPT)
}

pub fn live_inject_with(target: &Target, payload: &str) -> Result<(), String> {
    live_inject_with_dev(target, payload, None)
}

pub fn live_inject_with_dev(
    target: &Target,
    payload: &str,
    sink: Option<crate::inject::DevSink>,
) -> Result<(), String> {
    // Herdr targets go through its pane-owned PTY; raw windows use OS input.
    let mut inject_pid = |pid: u32, text: &str| inject_pid_dev(pid, text, sink.as_ref());
    crate::inject::execute_live_payload(target, payload, &mut inject_pid, &mut spawn_herdr)
}

pub fn command_hidden(bin: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new(bin);
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        c.creation_flags(CREATE_NO_WINDOW);
        c
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(bin)
    }
}

pub fn spawn_herdr(bin: &str, args: &[String]) -> Result<(), String> {
    match command_hidden(bin).args(args).output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
}

fn inject_pid_dev(
    pid: u32,
    text: &str,
    sink: Option<&crate::inject::DevSink>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        crate::inject_os_win::focus_and_type(pid, text, sink)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(sink) = sink {
            crate::inject::write_dev(
                sink,
                &serde_json::json!({"event":"inject-dev","pid": pid, "payload": text, "os":"unix"}),
            );
        }
        crate::linux::inject_text(pid, text).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(sink) = sink {
            crate::inject::write_dev(
                sink,
                &serde_json::json!({"event":"inject-dev","pid": pid, "payload": text, "os":"macos"}),
            );
        }
        crate::macos::inject_text(pid, text).map_err(|e| e.to_string())?;
        Ok(())
    }
}
