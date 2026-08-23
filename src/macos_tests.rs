//! macOS backend: cursor and keyboard input via CoreGraphics, focus via System Events.

#[repr(C)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreate(source: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn CGEventGetLocation(event: *mut core::ffi::c_void) -> CGPoint;
    fn CGEventCreateKeyboardEvent(
        source: *mut core::ffi::c_void,
        keycode: u16,
        keydown: bool,
    ) -> *mut core::ffi::c_void;
    fn CGEventKeyboardSetUnicodeString(
        event: *mut core::ffi::c_void,
        length: usize,
        string: *const u16,
    );
    fn CGEventPost(tap: u32, event: *mut core::ffi::c_void);
    fn CFRelease(cf: *mut core::ffi::c_void);
}

pub fn cursor_pos() -> Option<(i32, i32)> {
    unsafe {
        let ev = CGEventCreate(core::ptr::null_mut());
        if ev.is_null() {
            return None;
        }
        let p = CGEventGetLocation(ev);
        CFRelease(ev);
        Some((p.x as i32, p.y as i32))
    }
}

fn osa(script: &str) -> Option<String> {
    let out = std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn foreground_pid() -> Option<u32> {
    osa("tell application \"System Events\" to get unix id of first process whose frontmost is true")?
        .parse()
        .ok()
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
        title: osa("tell application \"System Events\" to get name of front window of first process whose frontmost is true")
            .unwrap_or_default(),
        procs,
    })
}

fn process_snapshot() -> Vec<crate::focus::Proc> {
    let Ok(out) = std::process::Command::new("ps")
        .args(["-axo", "pid=,ppid=,comm="])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let parent = fields.next()?.parse().ok()?;
            let name = fields.collect::<Vec<_>>().join(" ");
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

pub fn inject_text(_pid: u32, text: &str) -> std::io::Result<()> {
    unsafe {
        for ch in text.chars() {
            let mut utf16 = [0u16; 2];
            let units = ch.encode_utf16(&mut utf16);
            for keydown in [true, false] {
                let event = CGEventCreateKeyboardEvent(core::ptr::null_mut(), 0, keydown);
                if event.is_null() {
                    return Err(std::io::Error::other(
                        "CoreGraphics could not create keyboard event",
                    ));
                }
                CGEventKeyboardSetUnicodeString(event, units.len(), units.as_ptr());
                CGEventPost(0, event);
                CFRelease(event);
            }
        }
        for keydown in [true, false] {
            let event = CGEventCreateKeyboardEvent(core::ptr::null_mut(), 36, keydown);
            if event.is_null() {
                return Err(std::io::Error::other(
                    "CoreGraphics could not create Return event",
                ));
            }
            CGEventPost(0, event);
            CFRelease(event);
        }
    }
    Ok(())
}
