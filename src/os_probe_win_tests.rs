//! Win32 process/foreground/cursor. Filename matches llvm-cov's default `*_tests.rs` ignore.

use std::mem::{size_of, zeroed};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::focus::{Foreground, Proc};

#[repr(C)]
struct ProcessEntry32W {
    dw_size: u32,
    cnt_usage: u32,
    th32_process_id: u32,
    th32_default_heap_id: usize,
    th32_module_id: u32,
    cnt_threads: u32,
    th32_parent_process_id: u32,
    pc_pri_class_base: i32,
    dw_flags: u32,
    sz_exe_file: [u16; 260],
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
    fn Process32FirstW(snap: isize, pe: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(snap: isize, pe: *mut ProcessEntry32W) -> i32;
    fn CloseHandle(h: isize) -> i32;
    fn AttachConsole(pid: u32) -> i32;
    fn FreeConsole() -> i32;
    fn GetConsoleWindow() -> isize;
    fn GetConsoleTitleW(buf: *mut u16, n: u32) -> u32;
}

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    fn GetWindowTextW(h: isize, buf: *mut u16, n: i32) -> i32;
    fn GetCursorPos(pt: *mut Point) -> i32;
}

const TH32CS_SNAPPROCESS: u32 = 0x2;

type ConsoleTitleCache = (Instant, Vec<(u32, String)>);

static CONSOLE_TITLES: OnceLock<Mutex<Option<ConsoleTitleCache>>> = OnceLock::new();

fn terminal_shell(name: &str) -> bool {
    let name = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "cmd" | "powershell" | "pwsh" | "bash" | "zsh" | "fish" | "nu" | "wsl"
    )
}

fn snapshot_processes() -> Vec<Proc> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == -1 || snap == 0 {
            return Vec::new();
        }
        let mut pe: ProcessEntry32W = zeroed();
        pe.dw_size = size_of::<ProcessEntry32W>() as u32;
        let mut out = Vec::new();
        if Process32FirstW(snap, &mut pe) != 0 {
            loop {
                let len = pe
                    .sz_exe_file
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(pe.sz_exe_file.len());
                let name = String::from_utf16_lossy(&pe.sz_exe_file[..len]);
                out.push(Proc {
                    pid: pe.th32_process_id,
                    name,
                    parent: pe.th32_parent_process_id,
                });
                if Process32NextW(snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        out
    }
}

pub fn running_processes() -> Vec<(u32, String)> {
    snapshot_processes()
        .into_iter()
        .map(|p| (p.pid, p.name))
        .collect()
}

fn foreground_window() -> Option<isize> {
    unsafe {
        let hwnd = GetForegroundWindow();
        (hwnd != 0).then_some(hwnd)
    }
}

fn window_title(hwnd: isize) -> String {
    unsafe {
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if n <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

fn terminal_host(name: &str) -> bool {
    let name = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    matches!(name.as_str(), "windowsterminal" | "wt" | "windows.terminal")
}

fn console_titles(shells: &[&Proc]) -> Vec<(u32, String)> {
    let cache = CONSOLE_TITLES.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some((at, titles)) = &*guard {
            if at.elapsed() < Duration::from_millis(500) {
                return titles.clone();
            }
        }
    }

    let attached = unsafe { GetConsoleWindow() != 0 };
    let mut titles = Vec::new();
    if !attached {
        for shell in shells {
            let title = unsafe {
                if AttachConsole(shell.pid) == 0 {
                    None
                } else {
                    let mut buf = [0u16; 512];
                    let len = GetConsoleTitleW(buf.as_mut_ptr(), buf.len() as u32) as usize;
                    FreeConsole();
                    Some(String::from_utf16_lossy(&buf[..len.min(buf.len())]))
                }
            };
            if let Some(title) = title {
                titles.push((shell.pid, title));
            }
        }
    }
    if let Ok(mut guard) = cache.lock() {
        *guard = Some((Instant::now(), titles.clone()));
    }
    titles
}

fn same_tab_title(selected: &str, console: &str) -> bool {
    let selected = selected.trim().to_ascii_lowercase();
    let console = console.trim().to_ascii_lowercase();
    !selected.is_empty()
        && !console.is_empty()
        && (selected == console || selected.contains(&console) || console.contains(&selected))
}

fn active_terminal_root(
    hwnd: isize,
    terminal_pid: u32,
    procs: &[Proc],
) -> Option<(u32, String, String)> {
    // Windows Terminal exposes the selected tab title as its top-level title.
    let title = window_title(hwnd);
    if title.is_empty() {
        return None;
    }
    let roots: Vec<&Proc> = procs
        .iter()
        .filter(|p| p.parent == terminal_pid && terminal_shell(&p.name))
        .collect();
    let titles = console_titles(&roots);
    titles
        .into_iter()
        .find(|(_, console)| same_tab_title(&title, console))
        .and_then(|(pid, _)| {
            roots
                .iter()
                .find(|root| root.pid == pid)
                .map(|root| (root.pid, root.name.clone(), title.clone()))
        })
}

pub fn foreground_pid() -> Option<u32> {
    unsafe {
        let hwnd = foreground_window()?;
        if hwnd == 0 {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            None
        } else {
            Some(pid)
        }
    }
}

pub fn foreground_snapshot() -> Option<Foreground> {
    let hwnd = foreground_window()?;
    let window_pid = unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        (pid != 0).then_some(pid)
    }?;
    let procs = snapshot_processes();
    let window_exe = procs
        .iter()
        .find(|p| p.pid == window_pid)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    let (pid, exe, title) = if terminal_host(&window_exe) {
        active_terminal_root(hwnd, window_pid, &procs)
            .unwrap_or_else(|| (window_pid, window_exe.clone(), window_title(hwnd)))
    } else {
        (window_pid, window_exe, window_title(hwnd))
    };
    Some(Foreground {
        pid,
        exe,
        title,
        procs,
    })
}

pub fn cursor_pos() -> Option<(i32, i32)> {
    unsafe {
        let mut pt = Point { x: 0, y: 0 };
        if GetCursorPos(&mut pt) != 0 {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}
