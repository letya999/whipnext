use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::detect::Probe;

pub fn os_home(userprofile: Option<OsString>, home: Option<OsString>) -> PathBuf {
    userprofile
        .or(home)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub struct SystemProbe;

impl Probe for SystemProbe {
    fn path_dirs(&self) -> Vec<PathBuf> {
        std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default()
    }
    fn exists(&self, path: &Path) -> bool {
        path.is_file()
    }
    fn running(&self) -> Vec<(u32, String)> {
        running_processes()
    }
    fn home(&self) -> PathBuf {
        os_home(std::env::var_os("USERPROFILE"), std::env::var_os("HOME"))
    }
    fn local_app_data(&self) -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }
}

fn running_processes() -> Vec<(u32, String)> {
    #[cfg(windows)]
    {
        crate::os_probe_win::running_processes()
    }
    #[cfg(target_os = "macos")]
    {
        crate::macos::running_processes()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        crate::linux::running_processes()
    }
}

pub fn foreground_snapshot() -> Option<crate::focus::Foreground> {
    #[cfg(windows)]
    {
        crate::os_probe_win::foreground_snapshot()
    }
    #[cfg(target_os = "macos")]
    {
        crate::macos::foreground_snapshot()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        crate::linux::foreground_snapshot()
    }
    #[cfg(not(any(windows, unix)))]
    {
        None
    }
}

pub fn foreground_pid() -> Option<u32> {
    #[cfg(windows)]
    {
        crate::os_probe_win::foreground_pid()
    }
    #[cfg(target_os = "macos")]
    {
        crate::macos::foreground_pid()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        crate::linux::foreground_pid()
    }
    #[cfg(not(any(windows, unix)))]
    {
        None
    }
}

pub fn cursor_pos() -> Option<(i32, i32)> {
    #[cfg(windows)]
    {
        crate::os_probe_win::cursor_pos()
    }
    #[cfg(target_os = "macos")]
    {
        crate::macos::cursor_pos()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        crate::linux::cursor_pos()
    }
}
