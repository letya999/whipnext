//! Compose installed-agent catalog + Herdr list + grok sessions into a Target.

use crate::detect::{self, AgentStatus, Probe};
use crate::herdr::{self, HerdrAgent};
use crate::kind::Kind;
use crate::os_probe::SystemProbe;
use crate::session::{parse_active_sessions, select_current, GrokSession, Target};

pub fn grok_pids(catalog: &[AgentStatus]) -> Vec<u32> {
    catalog
        .iter()
        .find(|c| c.kind == Kind::Grok)
        .map(|c| c.pids.clone())
        .unwrap_or_default()
}

pub fn parse_herdr_output(ok: bool, stdout: &str) -> Option<Vec<HerdrAgent>> {
    if !ok {
        return None;
    }
    herdr::parse_agent_list(stdout).ok()
}

pub fn grok_sessions_from_json(raw: Option<&str>) -> Vec<GrokSession> {
    raw.and_then(|s| parse_active_sessions(s).ok())
        .unwrap_or_default()
}

/// Session pick with empty ancestor list — tests and simple probes.
pub fn resolve(
    herdr: Option<&[HerdrAgent]>,
    catalog: &[AgentStatus],
    fg: Option<u32>,
    grok: &[GrokSession],
) -> Option<Target> {
    select_current(herdr, catalog, fg, &[], grok, &grok_pids(catalog))
}

pub fn list_herdr_agents() -> Vec<HerdrAgent> {
    let out = crate::inject_os::command_hidden("herdr")
        .args(["agent", "list"])
        .output()
        .ok();
    out.and_then(|o| parse_herdr_output(o.status.success(), &String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

pub fn resolve_from_probe(
    probe: &dyn Probe,
    herdr_ok: bool,
    herdr_stdout: &str,
    grok_json: Option<&str>,
    fg: Option<u32>,
) -> Option<Target> {
    let herdr = parse_herdr_output(herdr_ok, herdr_stdout);
    let catalog = detect::catalog(probe);
    let grok = grok_sessions_from_json(grok_json);
    resolve(herdr.as_deref(), &catalog, fg, &grok)
}

/// Live I/O used by the overlay. Failures become `None` (no inject).
pub fn resolve_live_target() -> Option<Target> {
    let snap = crate::os_probe::foreground_snapshot();
    let fg = snap.as_ref().map(|f| f.pid);
    let ancestors: Vec<u32> = snap
        .as_ref()
        .map(crate::focus::related_pids)
        .unwrap_or_default();
    let herdr_list = list_herdr_agents();
    let catalog = detect::catalog(&SystemProbe);
    let grok_path = SystemProbe
        .home()
        .join(".grok")
        .join("active_sessions.json");
    let grok = grok_sessions_from_json(std::fs::read_to_string(grok_path).ok().as_deref());
    select_current(
        Some(herdr_list.as_slice()).filter(|v| !v.is_empty()),
        &catalog,
        fg,
        &ancestors,
        &grok,
        &grok_pids(&catalog),
    )
}
