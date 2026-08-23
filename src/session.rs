use crate::detect::AgentStatus;
use crate::herdr::{focused_agent, HerdrAgent};
use crate::kind::Kind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub kind: Kind,
    pub session_id: Option<String>,
    pub pid: Option<u32>,
    pub pane_id: Option<String>,
    pub herdr_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GrokSession {
    pub session_id: String,
    pub pid: u32,
    pub cwd: String,
}

/// Pick the live session that should receive `next`.
///
/// Order: agent pid that owns the focused window (or is in its process tree)
/// → focused Herdr pane when the window is not a direct agent pid
/// → grok `active_sessions.json` pid match.
pub fn select_current(
    herdr: Option<&[HerdrAgent]>,
    catalog: &[AgentStatus],
    foreground_pid: Option<u32>,
    ancestors_of_foreground: &[u32],
    grok_sessions: &[GrokSession],
    grok_alive: &[u32],
) -> Option<Target> {
    if let Some(t) = pid_target(catalog, foreground_pid, ancestors_of_foreground) {
        return Some(t);
    }

    if let Some(a) = herdr.and_then(focused_agent) {
        return Some(Target {
            kind: a.kind,
            session_id: a.session_id.clone(),
            pid: None,
            pane_id: Some(a.pane_id.clone()),
            herdr_name: a.name.clone(),
        });
    }

    let live: Vec<&GrokSession> = grok_sessions
        .iter()
        .filter(|s| grok_alive.contains(&s.pid))
        .collect();
    if let Some(fg) = foreground_pid {
        if let Some(s) = live.iter().find(|s| s.pid == fg) {
            return Some(Target {
                kind: Kind::Grok,
                session_id: Some(s.session_id.clone()),
                pid: Some(s.pid),
                pane_id: None,
                herdr_name: None,
            });
        }
    }
    None
}

fn pid_target(
    catalog: &[AgentStatus],
    foreground_pid: Option<u32>,
    ancestors_of_foreground: &[u32],
) -> Option<Target> {
    let fg = foreground_pid?;
    let mut matches: Vec<Target> = Vec::new();
    for st in catalog {
        for pid in &st.pids {
            if *pid == fg || ancestors_of_foreground.contains(pid) {
                matches.push(Target {
                    kind: st.kind,
                    session_id: None,
                    pid: Some(*pid),
                    pane_id: None,
                    herdr_name: None,
                });
            }
        }
    }
    if matches.len() == 1 {
        return matches.pop();
    }
    if let Some(exact) = matches.iter().find(|t| t.pid == Some(fg)) {
        return Some(exact.clone());
    }
    matches.into_iter().next()
}

/// After a crack, bind inject to the window that was focused at click.
pub fn retarget_for_foreground(
    mut target: Target,
    fg: Option<&crate::focus::Foreground>,
    settings: &crate::settings::Settings,
) -> Target {
    let Some(fg) = fg else {
        return target;
    };
    if crate::focus::is_self(&fg.exe) {
        return target;
    }
    let _ = settings;
    if target.pane_id.is_some() && crate::focus::is_host(&fg.exe) {
        target.pid = None;
        return target;
    }
    target.pid = Some(fg.pid);
    if !crate::focus::is_host(&fg.exe) {
        target.pane_id = None;
    }
    target
}

pub fn parse_active_sessions(json: &str) -> Result<Vec<GrokSession>, String> {
    #[derive(serde::Deserialize)]
    struct Row {
        session_id: String,
        pid: u32,
        cwd: String,
    }
    let rows: Vec<Row> =
        serde_json::from_str(json).map_err(|e| format!("active_sessions.json: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|r| GrokSession {
            session_id: r.session_id,
            pid: r.pid,
            cwd: r.cwd,
        })
        .collect())
}
