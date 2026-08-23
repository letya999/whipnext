#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
fn set_app_user_model_id() {
    #[link(name = "shell32")]
    extern "system" {
        #[link_name = "SetCurrentProcessExplicitAppUserModelID"]
        fn set_current_process_explicit_app_user_model_id(appid: *const u16) -> i32;
    }
    let id: Vec<u16> = "whipnext.app\0".encode_utf16().collect();
    unsafe {
        set_current_process_explicit_app_user_model_id(id.as_ptr());
    }
}

fn main() {
    #[cfg(windows)]
    set_app_user_model_id();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match whipnext::cli::classify(&args) {
        Ok(whipnext::cli::Action::SettingsUi) => whipnext::ui_host::run(),
        _ => whipnext::cli::dispatch(&args),
    };
    let code = whipnext::cli::status_code(result);
    if code != 0 {
        std::process::exit(code);
    }
}
