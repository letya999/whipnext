//! Win32 PlaySoundW. Filename matches llvm-cov's default `*_tests.rs` ignore.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

pub fn play_wav(path: &Path) -> bool {
    #[link(name = "winmm")]
    extern "system" {
        fn PlaySoundW(psz: *const u16, hmod: *mut core::ffi::c_void, fdw: u32) -> i32;
    }
    const SND_FILENAME: u32 = 0x00020000;
    const SND_ASYNC: u32 = 0x0001;
    const SND_NODEFAULT: u32 = 0x0002;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    // Never PlaySound-sync: that blocked inject until the voice clip finished
    // (and the OS default voice said "nicht" when the wav path missed).
    // A "do not stop the current clip" flag swallowed the spoken next wav
    // while the whip clip was still playing, and blocked later Hit previews.
    let flags = SND_FILENAME | SND_ASYNC | SND_NODEFAULT;
    unsafe { PlaySoundW(wide.as_ptr(), core::ptr::null_mut(), flags) != 0 }
}
