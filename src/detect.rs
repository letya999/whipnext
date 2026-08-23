use std::path::{Path, PathBuf};

use crate::kind::{extra_bin_dirs, Kind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentStatus {
    pub kind: Kind,
    pub installed: bool,
    pub install_path: Option<PathBuf>,
    pub running: bool,
    pub pids: Vec<u32>,
}

pub trait Probe {
    fn path_dirs(&self) -> Vec<PathBuf>;
    fn exists(&self, path: &Path) -> bool;
    fn running(&self) -> Vec<(u32, String)>;
    fn home(&self) -> PathBuf;
    fn local_app_data(&self) -> Option<PathBuf>;
}

pub fn catalog(probe: &dyn Probe) -> Vec<AgentStatus> {
    let mut search = probe.path_dirs();
    search.extend(extra_bin_dirs(
        &probe.home(),
        probe.local_app_data().as_deref(),
    ));
    let running = probe.running();
    Kind::ALL
        .into_iter()
        .map(|kind| {
            let mut install_path = None;
            for dir in &search {
                for bin in kind.binaries() {
                    let candidate = dir.join(bin);
                    if probe.exists(&candidate) {
                        install_path = Some(candidate);
                        break;
                    }
                }
                if install_path.is_some() {
                    break;
                }
            }
            let pids: Vec<u32> = running
                .iter()
                .filter_map(|(pid, name)| {
                    Kind::from_binary_name(name)
                        .filter(|k| *k == kind)
                        .map(|_| *pid)
                })
                .collect();
            AgentStatus {
                kind,
                installed: install_path.is_some(),
                install_path,
                running: !pids.is_empty(),
                pids,
            }
        })
        .collect()
}

#[derive(Default)]
pub struct FakeProbe {
    pub path_dirs: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
    pub running: Vec<(u32, String)>,
    pub home: PathBuf,
    pub local_app_data: Option<PathBuf>,
}

impl Probe for FakeProbe {
    fn path_dirs(&self) -> Vec<PathBuf> {
        self.path_dirs.clone()
    }
    fn exists(&self, path: &Path) -> bool {
        self.files.iter().any(|p| p == path)
    }
    fn running(&self) -> Vec<(u32, String)> {
        self.running.clone()
    }
    fn home(&self) -> PathBuf {
        self.home.clone()
    }
    fn local_app_data(&self) -> Option<PathBuf> {
        self.local_app_data.clone()
    }
}
