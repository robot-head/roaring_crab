use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum,
)]
#[clap(rename_all = "PascalCase")]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    Notification,
    Stop,
    SubagentStop,
    PreCompact,
}

impl HookEvent {
    pub const ALL: [HookEvent; 9] = [
        HookEvent::SessionStart,
        HookEvent::SessionEnd,
        HookEvent::UserPromptSubmit,
        HookEvent::PreToolUse,
        HookEvent::PostToolUse,
        HookEvent::Notification,
        HookEvent::Stop,
        HookEvent::SubagentStop,
        HookEvent::PreCompact,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            HookEvent::SessionStart => "SessionStart",
            HookEvent::SessionEnd => "SessionEnd",
            HookEvent::UserPromptSubmit => "UserPromptSubmit",
            HookEvent::PreToolUse => "PreToolUse",
            HookEvent::PostToolUse => "PostToolUse",
            HookEvent::Notification => "Notification",
            HookEvent::Stop => "Stop",
            HookEvent::SubagentStop => "SubagentStop",
            HookEvent::PreCompact => "PreCompact",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown hook event: {0}")]
pub struct UnknownHookEvent(pub String);

impl FromStr for HookEvent {
    type Err = UnknownHookEvent;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for h in Self::ALL {
            if h.as_str() == s {
                return Ok(h);
            }
        }
        Err(UnknownHookEvent(s.to_string()))
    }
}
