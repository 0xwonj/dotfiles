use std::io::{self, IsTerminal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Accent,
    Muted,
    Ok,
    Warn,
    Fail,
    Note,
}

#[derive(Debug, Clone, Copy)]
pub struct Ui {
    color: bool,
}

impl Ui {
    pub fn detect() -> Self {
        let color = match std::env::var("DOTCTL_COLOR") {
            Ok(value) if value.eq_ignore_ascii_case("always") => true,
            Ok(value) if value.eq_ignore_ascii_case("never") => false,
            _ => std::env::var_os("NO_COLOR").is_none() && io::stdout().is_terminal(),
        };
        Self { color }
    }

    pub fn section(&self, title: &str) -> String {
        let prefix = self.paint(Tone::Accent, "==>");
        let title = self.bold(title);
        format!("\n{prefix} {title}")
    }

    pub fn detail(&self, label: &str, value: &str) -> String {
        let label = self.paint(Tone::Muted, &format!("{label:<12}"));
        format!("  {label} {value}")
    }

    pub fn status(&self, label: &str, message: &str) -> String {
        let (tone, display) = match label {
            "ok" => (Tone::Ok, "OK"),
            "run" => (Tone::Accent, "RUN"),
            "skip" => (Tone::Muted, "SKIP"),
            "warn" => (Tone::Warn, "WARN"),
            "fail" => (Tone::Fail, "FAIL"),
            "miss" => (Tone::Fail, "MISS"),
            "note" => (Tone::Note, "NOTE"),
            _ => (Tone::Muted, label),
        };
        let badge = format!("[ {display:>4} ]");
        let badge = self.paint(tone, &badge);
        format!("  {badge} {message}")
    }

    pub fn list_item(&self, label: &str, description: &str) -> String {
        let label = self.bold(label);
        let description = self.paint(Tone::Muted, description);
        format!("  {label:<16} {description}")
    }

    pub fn summary(&self, title: &str, ok: bool) -> String {
        let tone = if ok { Tone::Ok } else { Tone::Fail };
        let badge = if ok { "READY" } else { "CHECK" };
        let badge = self.paint(tone, &format!("[ {badge} ]"));
        let title = self.bold(title);
        format!("\n{badge} {title}")
    }

    fn bold(&self, value: &str) -> String {
        if self.color {
            format!("\x1b[1m{value}\x1b[0m")
        } else {
            value.to_string()
        }
    }

    fn paint(&self, tone: Tone, value: &str) -> String {
        if !self.color {
            return value.to_string();
        }
        let code = match tone {
            Tone::Accent => "36",
            Tone::Muted => "2",
            Tone::Ok => "32",
            Tone::Warn => "33",
            Tone::Fail => "31",
            Tone::Note => "35",
        };
        format!("\x1b[{code}m{value}\x1b[0m")
    }
}
