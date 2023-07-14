use colored::Colorize;
use cronback_api_model::TriggerStatus;
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

/// Respects NO_COLOR environment variable to avoid showing emojis if tty can't
/// display them.
pub fn emoji(s: &str) -> String {
    if *SHOULD_COLORIZE {
        format!("{} ", s)
    } else {
        String::new()
    }
}
