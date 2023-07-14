use colored::Colorize;
use cronback_api_model::{AttemptStatus, RunStatus, TriggerStatus};
use once_cell::sync::Lazy;

static SHOULD_COLORIZE: Lazy<bool> = Lazy::new(|| {
    colored::control::ShouldColorize::from_env().should_colorize()
});

pub trait FancyToString {
    fn fancy(&self) -> String;
}

/// Convenience to enable fancy() on Option<T>
impl<T> FancyToString for Option<T>
where
    T: FancyToString,
{
    fn fancy(&self) -> String {
        match self {
            | Some(t) => t.fancy(),
            | None => "".to_string(),
        }
    }
}

/// Respects NO_COLOR environment variable to avoid showing emojis if tty can't
/// display them.
pub fn emoji(s: &str) -> String {
    if *SHOULD_COLORIZE {
        format!("{} ", s)
    } else {
        String::new()
    }
}

// --- Fancy for specific types
impl FancyToString for TriggerStatus {
    fn fancy(&self) -> String {
        match self {
            | TriggerStatus::Scheduled => {
                format!("{}{}", emoji("⏰"), self.to_string().green())
            }
            | TriggerStatus::OnDemand => format!("{}{self}", emoji("📍")),
            | TriggerStatus::Expired => {
                format!("{}{}", emoji("〰"), self.to_string().italic())
            }
            | TriggerStatus::Cancelled => format!("{}{self}", emoji("✖️")),
            | TriggerStatus::Paused => {
                format!("{}{}", emoji("🔸"), self.to_string().blink())
            }
            | s => s.to_string(),
        }
    }
}

impl FancyToString for RunStatus {
    fn fancy(&self) -> String {
        match self {
            | RunStatus::Attempting => {
                format!("{}{}", emoji("🚤"), self.to_string().yellow())
            }
            | RunStatus::Failed => {
                format!("{}{}", emoji("❌"), self.to_string().red())
            }
            | RunStatus::Succeeded => {
                format!("{}{}", emoji("✅"), self.to_string().green())
            }
            | s => s.to_string(),
        }
    }
}

impl FancyToString for AttemptStatus {
    fn fancy(&self) -> String {
        match self {
            | AttemptStatus::Failed => {
                format!("{}{}", emoji("❌"), self.to_string().red())
            }
            | AttemptStatus::Succeeded => {
                format!("{}{}", emoji("✅"), self.to_string().green())
            }
            | s => s.to_string(),
        }
    }
}
