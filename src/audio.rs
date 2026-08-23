use std::path::PathBuf;

pub fn assets_dir_from(
    env_assets: Option<String>,
    exe: Option<PathBuf>,
    fallback: PathBuf,
) -> PathBuf {
    if let Some(d) = env_assets {
        return PathBuf::from(d);
    }
    if let Some(dir) = exe.as_ref().and_then(|e| e.parent()) {
        let beside = dir.join("assets");
        if beside.join("pack").is_dir() || beside.join("frames").is_dir() {
            return beside;
        }
    }
    fallback
}

pub fn assets_dir() -> PathBuf {
    assets_dir_from(
        std::env::var("WHIPNEXT_ASSETS").ok(),
        std::env::current_exe().ok(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
    )
}

pub fn usable_wav(path: &std::path::Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    bytes.len() >= 44 && bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WAVE"
}

pub fn sound_file(stem: &str) -> PathBuf {
    let catalog = crate::pack::catalog_dir(&crate::settings::home_dir())
        .join("sounds")
        .join(format!("{stem}.wav"));
    if usable_wav(&catalog) {
        return catalog;
    }
    let pack = assets_dir()
        .join("pack")
        .join("sounds")
        .join(format!("{stem}.wav"));
    if usable_wav(&pack) {
        pack
    } else {
        assets_dir().join("sounds").join(format!("{stem}.wav"))
    }
}

pub fn whip_wav() -> PathBuf {
    sound_file("whip")
}

pub fn next_wav() -> PathBuf {
    sound_file("next")
}

pub fn play_wav(path: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        crate::audio_win::play_wav(path)
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("afplay")
            .arg(path)
            .spawn()
            .is_ok()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if std::process::Command::new("paplay")
            .arg(path)
            .spawn()
            .is_ok()
        {
            true
        } else {
            std::process::Command::new("aplay")
                .arg(path)
                .spawn()
                .is_ok()
        }
    }
}

pub struct OsAudio {
    pub whip: String,
    pub next: String,
}

impl Default for OsAudio {
    fn default() -> Self {
        Self {
            whip: "whip".into(),
            next: "next".into(),
        }
    }
}

impl crate::whip::Audio for OsAudio {
    fn play_whip(&mut self) {
        if !play_wav(&sound_file(&self.whip)) && self.whip != "whip" {
            let _ = play_wav(&whip_wav());
        }
    }
    fn play_next(&mut self) {
        play_wav(&sound_file(&self.next));
    }
}
