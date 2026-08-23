//! Linux backend: X11 cursor/focus via xdotool, process inspection via /proc.

use std::io::Write;
use std::path::PathBuf;

pub fn cursor_pos() -> Option<(i32, i32)> {
    let out = std::process::Command::new("xdotool")
        .args(["getmouselocation", "--shell"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut x = None;
    let mut y = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("X=") {
            x = v.trim().parse().ok();
        }
        if let Some(v) = line.strip_prefix("Y=") {
            y = v.trim().parse().ok();
        }
    }
    out.status.success().then_some((x?, y?))
}

pub fn tty_path(pid: u32) -> PathBuf {
    PathBuf::from(format!("/proc/{pid}/fd/0"))
}

/// Write the user phrase plus newline to the process controlling tty.
pub fn inject_text(pid: u32, text: &str) -> std::io::Result<()> {
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .open(tty_path(pid))
        .map_err(|e| std::io::Error::new(e.kind(), format!("pid {pid}: {e}")))?;
    f.write_all(text.as_bytes())?;
    f.write_all(b"\n")
}

fn xdotool(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("xdotool")
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn foreground_pid() -> Option<u32> {
    xdotool(&["getactivewindow", "getwindowpid"])?.parse().ok()
}

pub fn foreground_snapshot() -> Option<crate::focus::Foreground> {
    let pid = foreground_pid()?;
    let procs = process_snapshot();
    let exe = procs
        .iter()
        .find(|p| p.pid == pid)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    Some(crate::focus::Foreground {
        pid,
        exe,
        title: xdotool(&["getactivewindow", "getwindowname"]).unwrap_or_default(),
        procs,
    })
}

pub fn process_snapshot() -> Vec<crate::focus::Proc> {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    dir.flatten()
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse().ok()?;
            let name = std::fs::read_to_string(entry.path().join("comm"))
                .ok()?
                .trim()
                .to_string();
            let status = std::fs::read_to_string(entry.path().join("status")).ok()?;
            let parent = status
                .lines()
                .find_map(|line| line.strip_prefix("PPid:")?.trim().parse().ok())
                .unwrap_or_default();
            Some(crate::focus::Proc { pid, name, parent })
        })
        .collect()
}

pub fn running_processes() -> Vec<(u32, String)> {
    process_snapshot()
        .into_iter()
        .map(|p| (p.pid, p.name))
        .collect()
}
