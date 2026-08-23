use serde::Deserialize;

use crate::kind::Kind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HerdrAgent {
    pub kind: Kind,
    pub name: Option<String>,
    pub pane_id: String,
    pub focused: bool,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Deserialize)]
struct Envelope {
    result: AgentListResult,
}

#[derive(Deserialize)]
struct AgentListResult {
    #[serde(default)]
    agents: Vec<RawAgent>,
}

#[derive(Deserialize)]
struct RawAgent {
    agent: String,
    #[serde(default)]
    name: Option<String>,
    pane_id: String,
    #[serde(default)]
    focused: bool,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    agent_session: Option<RawSession>,
}

#[derive(Deserialize)]
struct RawSession {
    #[serde(default)]
    value: Option<String>,
}

pub fn parse_agent_list(json: &str) -> Result<Vec<HerdrAgent>, String> {
    let env: Envelope =
        serde_json::from_str(json).map_err(|e| format!("herdr agent list json: {e}"))?;
    let mut out = Vec::new();
    for raw in env.result.agents {
        let Some(kind) = Kind::from_herdr(&raw.agent) else {
            continue;
        };
        out.push(HerdrAgent {
            kind,
            name: raw.name,
            pane_id: raw.pane_id,
            focused: raw.focused,
            session_id: raw.agent_session.and_then(|s| s.value),
            cwd: raw.cwd,
        });
    }
    Ok(out)
}

pub fn focused_agent(agents: &[HerdrAgent]) -> Option<&HerdrAgent> {
    agents.iter().find(|a| a.focused)
}

/// Shipped argv for sending `next` into a Herdr pane/agent.
pub fn prompt_next_args(target: &str) -> Vec<String> {
    prompt_args(target, "next")
}

pub fn prompt_args(target: &str, phrase: &str) -> Vec<String> {
    vec![
        "agent".into(),
        "prompt".into(),
        target.into(),
        crate::inject::sanitize_phrase(phrase),
    ]
}

pub const HERDR_BIN: &str = "herdr";
