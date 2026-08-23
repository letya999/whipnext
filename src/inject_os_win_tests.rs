//! Win32 SendInput / window focus. Filename matches llvm-cov's default `*_tests.rs` ignore.

use std::mem::{size_of, zeroed};

const KEYEVENTF_UNICODE: u32 = 0x0004;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const KEYEVENTF_SCANCODE: u32 = 0x0008;
const VK_RETURN: u16 = 0x0D;
const INPUT_KEYBOARD: u32 = 1;
const SW_RESTORE: i32 = 9;
const TH32CS_SNAPPROCESS: u32 = 0x2;
const ASFW_ANY: u32 = u32::MAX;

#[repr(C)]
#[derive(Clone, Copy)]
struct KeybdInput {
    w_vk: u16,
    w_scan: u16,
    dw_flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    dw_flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
union InputUnion {
    ki: KeybdInput,
    mi: MouseInput,
}

#[repr(C)]
struct Input {
    ty: u32,
    u: InputUnion,
}

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

#[link(name = "user32")]
extern "system" {
    fn SendInput(n: u32, p: *const Input, cb: i32) -> u32;
    fn SetForegroundWindow(hwnd: isize) -> i32;
    fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    fn AttachThreadInput(id_attach: u32, id_attach_to: u32, attach: i32) -> i32;
    fn EnumWindows(
        cb: Option<unsafe extern "system" fn(isize, isize) -> i32>,
        lparam: isize,
    ) -> i32;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn AllowSetForegroundWindow(pid: u32) -> i32;
    fn MapVirtualKeyW(code: u32, map_type: u32) -> u32;
    fn IsIconic(hwnd: isize) -> i32;
    fn GetAsyncKeyState(v: i32) -> i16;
    fn GetKeyboardLayout(id_thread: u32) -> usize;
    fn VkKeyScanExW(ch: u16, layout: usize) -> i16;
    fn OpenClipboard(h: isize) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(fmt: u32, mem: isize) -> isize;
    fn BlockInput(block: i32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> u32;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
    fn Process32FirstW(snap: isize, pe: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(snap: isize, pe: *mut ProcessEntry32W) -> i32;
    fn CloseHandle(h: isize) -> i32;
    fn Sleep(ms: u32);
    fn GlobalAlloc(flags: u32, bytes: usize) -> isize;
    fn GlobalLock(h: isize) -> *mut u8;
    fn GlobalUnlock(h: isize) -> i32;
}

struct EnumState {
    want: u32,
    hwnd: isize,
}

unsafe extern "system" fn enum_cb(hwnd: isize, lp: isize) -> i32 {
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    let st = &mut *(lp as *mut EnumState);
    if pid == st.want {
        st.hwnd = hwnd;
        return 0;
    }
    1
}

fn hwnd_for_pid(pid: u32) -> Option<isize> {
    let mut st = EnumState { want: pid, hwnd: 0 };
    unsafe {
        EnumWindows(Some(enum_cb), &mut st as *mut _ as isize);
    }
    if st.hwnd != 0 {
        Some(st.hwnd)
    } else {
        None
    }
}

fn parent_pid(pid: u32) -> Option<u32> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == 0 || snap == -1 {
            return None;
        }
        let mut pe: ProcessEntry32W = zeroed();
        pe.dw_size = size_of::<ProcessEntry32W>() as u32;
        let mut found = None;
        if Process32FirstW(snap, &mut pe) != 0 {
            loop {
                if pe.th32_process_id == pid {
                    found = Some(pe.th32_parent_process_id);
                    break;
                }
                if Process32NextW(snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        found
    }
}

fn hwnd_climbing(mut pid: u32) -> Option<isize> {
    let mut seen = std::collections::HashSet::new();
    for _ in 0..8 {
        if !seen.insert(pid) {
            break;
        }
        if let Some(h) = hwnd_for_pid(pid) {
            return Some(h);
        }
        pid = parent_pid(pid)?;
    }
    None
}

fn send_unicode(ch: char) -> Result<(), String> {
    let scan = ch as u32 as u16;
    let extra = 0usize;
    let down = Input {
        ty: INPUT_KEYBOARD,
        u: InputUnion {
            ki: KeybdInput {
                w_vk: 0,
                w_scan: scan,
                dw_flags: KEYEVENTF_UNICODE,
                time: 0,
                extra,
            },
        },
    };
    let up = Input {
        ty: INPUT_KEYBOARD,
        u: InputUnion {
            ki: KeybdInput {
                w_vk: 0,
                w_scan: scan,
                dw_flags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                time: 0,
                extra,
            },
        },
    };
    let sent = unsafe { SendInput(1, &down, size_of::<Input>() as i32) };
    if sent != 1 {
        return Err(format!("SendInput key-down failed ({sent}/1)"));
    }
    unsafe {
        Sleep(crate::inject::INJECT_DOWN_UP_MS);
    }
    let sent = unsafe { SendInput(1, &up, size_of::<Input>() as i32) };
    if sent == 1 {
        Ok(())
    } else {
        Err(format!("SendInput key-up failed ({sent}/1)"))
    }
}

fn send_key(vk: u16, scan: u16, flags: u32) -> Result<(), String> {
    let ev = key_input(vk, scan, flags);
    send_inputs(&[ev])
}

fn key_input(vk: u16, scan: u16, flags: u32) -> Input {
    Input {
        ty: INPUT_KEYBOARD,
        u: InputUnion {
            ki: KeybdInput {
                w_vk: vk,
                w_scan: scan,
                dw_flags: flags,
                time: 0,
                extra: 0,
            },
        },
    }
}

fn send_inputs(events: &[Input]) -> Result<(), String> {
    let sent = unsafe {
        SendInput(
            events.len() as u32,
            events.as_ptr(),
            size_of::<Input>() as i32,
        )
    };
    if sent == events.len() as u32 {
        Ok(())
    } else {
        Err(format!("SendInput failed ({sent}/{})", events.len()))
    }
}

fn send_vk_scan(vk: u16) -> Result<(), String> {
    let scan = unsafe { MapVirtualKeyW(vk as u32, 0) as u16 };
    send_key(vk, scan, KEYEVENTF_SCANCODE)?;
    unsafe {
        Sleep(crate::inject::INJECT_DOWN_UP_MS);
    }
    send_key(vk, scan, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP)
}

fn key_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

struct InputBlock;

impl InputBlock {
    fn while_modifier_held() -> Result<Option<Self>, String> {
        if !crate::inject::MODIFIER_VKS.into_iter().any(key_down) {
            return Ok(None);
        }
        if unsafe { BlockInput(1) } == 0 {
            return Err("BlockInput failed while a modifier is held".into());
        }
        Ok(Some(Self))
    }
}

impl Drop for InputBlock {
    fn drop(&mut self) {
        unsafe {
            BlockInput(0);
        }
    }
}

const KEYEVENTF_EXTENDEDKEY: u32 = 0x0001;

fn modifier_ups() -> Vec<Input> {
    crate::inject::MODIFIER_VKS
        .into_iter()
        .map(|vk| {
            let extended = matches!(vk, 0xA3 | 0xA5 | 0x5B | 0x5C);
            key_input(
                vk,
                0,
                KEYEVENTF_KEYUP | if extended { KEYEVENTF_EXTENDEDKEY } else { 0 },
            )
        })
        .collect()
}

fn release_held_modifiers() -> Result<(), String> {
    for vk in crate::inject::MODIFIER_VKS {
        if key_down(vk) {
            let extended = matches!(vk, 0xA3 | 0xA5 | 0x5B | 0x5C);
            send_key(
                vk,
                0,
                KEYEVENTF_KEYUP | if extended { KEYEVENTF_EXTENDEDKEY } else { 0 },
            )?;
        }
    }
    Ok(())
}

const CF_UNICODETEXT: u32 = 13;
const GMEM_MOVEABLE: u32 = 0x0002;
const VK_SHIFT: u16 = 0x10;
const VK_CONTROL: u16 = 0x11;
const VK_V: u16 = 0x56;

fn layout_lookup(ch: char, layout: usize) -> Option<(u16, bool)> {
    let packed = unsafe { VkKeyScanExW(ch as u32 as u16, layout) };
    crate::inject::layout_key(packed)
}

fn send_layout_char(vk: u16, shift: bool) -> Result<(), String> {
    if shift {
        send_key(VK_SHIFT, 0, 0)?;
        unsafe {
            Sleep(crate::inject::INJECT_DOWN_UP_MS);
        }
    }
    let result = send_vk_scan(vk);
    if shift {
        return result.and(send_key(VK_SHIFT, 0, KEYEVENTF_KEYUP));
    }
    result
}

fn send_paste() -> Result<(), String> {
    let mut events = modifier_ups();
    events.extend([
        key_input(VK_CONTROL, 0, 0),
        key_input(VK_SHIFT, 0, 0),
        key_input(VK_V, 0, 0),
        key_input(VK_V, 0, KEYEVENTF_KEYUP),
        key_input(VK_SHIFT, 0, KEYEVENTF_KEYUP),
        key_input(VK_CONTROL, 0, KEYEVENTF_KEYUP),
    ]);
    send_inputs(&events)
}

fn send_submit() -> Result<(), String> {
    let scan = unsafe { MapVirtualKeyW(VK_RETURN as u32, 0) as u16 };
    let mut events = modifier_ups();
    events.extend([
        key_input(VK_RETURN, scan, KEYEVENTF_SCANCODE),
        key_input(VK_RETURN, scan, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP),
    ]);
    send_inputs(&events)
}

fn clipboard_wide(text: &str) -> Vec<u16> {
    let mut w: Vec<u16> = text.encode_utf16().collect();
    w.push(0);
    w
}

fn set_clipboard_text(text: &str) -> Result<(), String> {
    unsafe {
        let mut opened = false;
        for _ in 0..8 {
            if OpenClipboard(0) != 0 {
                opened = true;
                break;
            }
            Sleep(15);
        }
        if !opened {
            return Err("OpenClipboard failed".into());
        }
        if EmptyClipboard() == 0 {
            CloseClipboard();
            return Err("EmptyClipboard failed".into());
        }
        let wide = clipboard_wide(text);
        let bytes = wide.len() * 2;
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if mem == 0 {
            CloseClipboard();
            return Err("GlobalAlloc failed".into());
        }
        let p = GlobalLock(mem);
        if p.is_null() {
            CloseClipboard();
            return Err("GlobalLock failed".into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, p, bytes);
        GlobalUnlock(mem);
        if SetClipboardData(CF_UNICODETEXT, mem) == 0 {
            CloseClipboard();
            return Err("SetClipboardData failed".into());
        }
        CloseClipboard();
        Ok(())
    }
}

fn pid_related(a: u32, b: u32) -> bool {
    if a == 0 || b == 0 {
        return false;
    }
    if a == b {
        return true;
    }
    let climbs = |mut p: u32, want: u32| {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..8 {
            if p == want {
                return true;
            }
            if !seen.insert(p) {
                return false;
            }
            match parent_pid(p) {
                Some(n) if n != 0 => p = n,
                _ => return false,
            }
        }
        false
    };
    climbs(a, b) || climbs(b, a)
}

pub fn focus_and_type(
    pid: u32,
    text: &str,
    sink: Option<&crate::inject::DevSink>,
) -> Result<(), String> {
    unsafe {
        AllowSetForegroundWindow(ASFW_ANY);
        let fg = GetForegroundWindow();
        let mut fg_pid = 0u32;
        let fg_tid = GetWindowThreadProcessId(fg, &mut fg_pid);
        let hwnd = if fg != 0 && pid_related(fg_pid, pid) {
            fg
        } else {
            hwnd_climbing(pid).ok_or_else(|| format!("no window for pid {pid}"))?
        };
        let mut tgt_pid = 0u32;
        let tgt_tid = GetWindowThreadProcessId(hwnd, &mut tgt_pid);
        let our = GetCurrentThreadId();
        let already = hwnd == fg;
        if !already {
            if fg_tid != 0 {
                AttachThreadInput(our, fg_tid, 1);
            }
            if tgt_tid != 0 && tgt_tid != fg_tid {
                AttachThreadInput(our, tgt_tid, 1);
            }
            if crate::inject::focus_needs_restore(IsIconic(hwnd) != 0) {
                ShowWindow(hwnd, SW_RESTORE);
            }
            SetForegroundWindow(hwnd);
            Sleep(crate::inject::INJECT_ATTACH_MS);
        }
        let result = (|| -> Result<(), String> {
            Sleep(crate::inject::INJECT_SETTLE_MS);
            let _input_block = InputBlock::while_modifier_held()?;
            release_held_modifiers()?;
            Sleep(crate::inject::INJECT_MODIFIER_MS);
            let layout = GetKeyboardLayout(tgt_tid);
            let layout_ok = crate::inject::layout_ok_for(text, |ch| layout_lookup(ch, layout));
            let method = crate::inject::type_method(text, layout_ok);
            let mut paste_ok = None;
            let paste_err: Option<String> = None;
            let mut chars: Vec<serde_json::Value> = Vec::new();
            match method {
                crate::inject::TypeMethod::Keys => {
                    for key in crate::inject::typed_keys(text) {
                        match key {
                            crate::inject::TypedKey::Vk(vk) => {
                                chars.push(serde_json::json!({"kind":"vk","vk": vk}));
                                send_vk_scan(vk)?;
                            }
                            crate::inject::TypedKey::Unicode(ch) => {
                                chars.push(
                                    serde_json::json!({"kind":"unicode","ch": ch.to_string()}),
                                );
                                send_unicode(ch)?;
                            }
                        }
                        Sleep(crate::inject::INJECT_KEY_MS);
                    }
                }
                crate::inject::TypeMethod::Layout => {
                    for ch in text.chars() {
                        if let Some((vk, shift)) = layout_lookup(ch, layout) {
                            chars.push(serde_json::json!({
                                "kind":"layout","ch": ch.to_string(),"vk": vk,"shift": shift
                            }));
                            send_layout_char(vk, shift)?;
                        } else {
                            chars.push(serde_json::json!({"kind":"unicode","ch": ch.to_string()}));
                            send_unicode(ch)?;
                        }
                        Sleep(crate::inject::INJECT_KEY_MS);
                    }
                }
                crate::inject::TypeMethod::Paste => match set_clipboard_text(text) {
                    Ok(()) => {
                        paste_ok = Some(true);
                        chars.push(serde_json::json!({"kind":"paste","n": text.chars().count()}));
                        let paste_result = send_paste();
                        if paste_result.is_ok() {
                            Sleep(crate::inject::INJECT_PASTE_MS);
                        }
                        paste_result?;
                    }
                    Err(e) => {
                        return Err(format!("clipboard paste failed: {e}"));
                    }
                },
            }
            Sleep(crate::inject::INJECT_ENTER_MS);
            send_submit()?;
            let enter_sent = true;
            if let Some(sink) = sink {
                crate::inject::write_dev(
                    sink,
                    &serde_json::json!({
                        "event": "inject-dev",
                        "payload": text,
                        "method": format!("{method:?}").to_lowercase(),
                        "layout_ok": layout_ok,
                        "layout": format!("{layout:x}"),
                        "pid": pid,
                        "hwnd": hwnd,
                        "fg_pid": fg_pid,
                        "already": already,
                        "paste_ok": paste_ok,
                        "paste_err": paste_err,
                        "enter_sent": enter_sent,
                        "chars": chars,
                    }),
                );
            }
            Ok(())
        })();
        if !already {
            if tgt_tid != 0 && tgt_tid != fg_tid {
                AttachThreadInput(our, tgt_tid, 0);
            }
            if fg_tid != 0 {
                AttachThreadInput(our, fg_tid, 0);
            }
        }
        result
    }
}
