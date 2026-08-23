pub mod audio;
pub mod cli;
pub mod detect;
pub mod focus;
pub mod gltf;
pub mod herdr;
pub mod inject;
pub mod inject_os;
pub mod kind;
pub mod layout;
pub mod live;
pub mod os_probe;
pub mod overlay;
pub mod pack;
pub mod session;
pub mod settings;
pub mod ui;
pub mod video;
pub mod whip;

#[cfg(windows)]
#[path = "audio_win_tests.rs"]
mod audio_win;
#[cfg(windows)]
#[path = "inject_os_win_tests.rs"]
mod inject_os_win;
#[cfg(all(unix, not(target_os = "macos")))]
#[path = "linux_tests.rs"]
pub mod linux;
#[cfg(target_os = "macos")]
#[path = "macos_tests.rs"]
pub mod macos;
#[cfg(windows)]
#[path = "os_probe_win_tests.rs"]
mod os_probe_win;
#[cfg(not(windows))]
#[path = "overlay_unix_tests.rs"]
pub mod overlay_unix;
#[cfg(windows)]
#[path = "overlay_win_tests.rs"]
pub mod overlay_win;
#[path = "ui_host_tests.rs"]
pub mod ui_host;
