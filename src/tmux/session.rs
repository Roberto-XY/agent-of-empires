//! tmux session management

use anyhow::{bail, Result};
use std::process::Command;

use super::{session_exists_from_cache, SESSION_PREFIX};
use crate::session::Status;

pub struct Session {
    name: String,
}

impl Session {
    pub fn new(id: &str, title: &str) -> Result<Self> {
        Ok(Self {
            name: Self::generate_name(id, title),
        })
    }

    pub fn generate_name(id: &str, title: &str) -> String {
        let safe_title = sanitize_session_name(title);
        let short_id = if id.len() > 8 { &id[..8] } else { id };
        format!("{}{}_{}", SESSION_PREFIX, safe_title, short_id)
    }

    pub fn exists(&self) -> bool {
        // Try cache first
        if let Some(exists) = session_exists_from_cache(&self.name) {
            return exists;
        }

        // Fallback to direct check
        Command::new("tmux")
            .args(["has-session", "-t", &self.name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn create(&self, working_dir: &str, command: Option<&str>) -> Result<()> {
        if self.exists() {
            return Ok(());
        }

        let mut args = vec![
            "new-session".to_string(),
            "-d".to_string(),
            "-s".to_string(),
            self.name.clone(),
            "-c".to_string(),
            working_dir.to_string(),
        ];

        if let Some(cmd) = command {
            args.push(cmd.to_string());
        }

        let output = Command::new("tmux")
            .args(&args)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to create tmux session: {}", stderr);
        }

        // Register in cache
        super::refresh_session_cache();

        Ok(())
    }

    pub fn kill(&self) -> Result<()> {
        if !self.exists() {
            return Ok(());
        }

        let output = Command::new("tmux")
            .args(["kill-session", "-t", &self.name])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to kill tmux session: {}", stderr);
        }

        Ok(())
    }

    pub fn attach(&self) -> Result<()> {
        if !self.exists() {
            bail!("Session does not exist: {}", self.name);
        }

        // Check if we're already in tmux
        if std::env::var("TMUX").is_ok() {
            // Switch to session
            let status = Command::new("tmux")
                .args(["switch-client", "-t", &self.name])
                .status()?;

            if !status.success() {
                bail!("Failed to switch to tmux session");
            }
        } else {
            // Attach to session
            let status = Command::new("tmux")
                .args(["attach-session", "-t", &self.name])
                .status()?;

            if !status.success() {
                bail!("Failed to attach to tmux session");
            }
        }

        Ok(())
    }

    pub fn capture_pane(&self, lines: usize) -> Result<String> {
        if !self.exists() {
            return Ok(String::new());
        }

        let output = Command::new("tmux")
            .args([
                "capture-pane",
                "-t",
                &self.name,
                "-p",
                "-S",
                &format!("-{}", lines),
            ])
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Ok(String::new())
        }
    }

    pub fn detect_status(&self, tool: &str) -> Result<Status> {
        let content = self.capture_pane(50)?;
        Ok(detect_status_from_content(&content, tool))
    }

}

fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(20)
        .collect()
}

fn detect_status_from_content(content: &str, tool: &str) -> Status {
    let lines: Vec<&str> = content.lines().collect();
    let last_lines = if lines.len() > 10 {
        &lines[lines.len() - 10..]
    } else {
        &lines
    };
    let last_content = last_lines.join("\n").to_lowercase();

    match tool {
        "claude" => detect_claude_status(&last_content),
        "gemini" => detect_gemini_status(&last_content),
        "opencode" | "codex" => detect_generic_status(&last_content),
        _ => detect_shell_status(&last_content),
    }
}

fn detect_claude_status(content: &str) -> Status {
    // Claude waiting for input patterns
    let waiting_patterns = [
        "waiting for your input",
        "what would you like",
        "how can i help",
        "ready for your",
        "> ", // Prompt indicator
        "claude>",
    ];

    // Claude running patterns
    let running_patterns = [
        "thinking",
        "processing",
        "working on",
        "analyzing",
        "generating",
        "writing",
        "reading",
        "searching",
    ];

    // Error patterns
    let error_patterns = [
        "error:",
        "failed:",
        "exception:",
        "traceback",
        "panic:",
    ];

    for pattern in &error_patterns {
        if content.contains(pattern) {
            return Status::Error;
        }
    }

    for pattern in &running_patterns {
        if content.contains(pattern) {
            return Status::Running;
        }
    }

    for pattern in &waiting_patterns {
        if content.contains(pattern) {
            return Status::Waiting;
        }
    }

    Status::Idle
}

fn detect_gemini_status(content: &str) -> Status {
    let waiting_patterns = [
        "gemini>",
        "> ",
        "enter your",
        "type your",
    ];

    let running_patterns = [
        "generating",
        "thinking",
        "processing",
    ];

    for pattern in &running_patterns {
        if content.contains(pattern) {
            return Status::Running;
        }
    }

    for pattern in &waiting_patterns {
        if content.contains(pattern) {
            return Status::Waiting;
        }
    }

    Status::Idle
}

fn detect_generic_status(content: &str) -> Status {
    let running_patterns = [
        "running",
        "processing",
        "loading",
        "thinking",
    ];

    for pattern in &running_patterns {
        if content.contains(pattern) {
            return Status::Running;
        }
    }

    // Check for common prompts
    if content.ends_with("$ ") || content.ends_with("> ") || content.ends_with("# ") {
        return Status::Waiting;
    }

    Status::Idle
}

fn detect_shell_status(content: &str) -> Status {
    // Shell prompts
    if content.ends_with("$ ") || content.ends_with("> ") || content.ends_with("# ") || content.ends_with("% ") {
        return Status::Waiting;
    }

    // Running if we see a spinner or progress indicator
    let running_indicators = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "...", "───"];
    for indicator in &running_indicators {
        if content.contains(indicator) {
            return Status::Running;
        }
    }

    Status::Idle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_session_name() {
        assert_eq!(sanitize_session_name("my-project"), "my-project");
        assert_eq!(sanitize_session_name("my project"), "my_project");
        assert_eq!(sanitize_session_name("a".repeat(30).as_str()).len(), 20);
    }

    #[test]
    fn test_generate_name() {
        let name = Session::generate_name("abc123def456", "My Project");
        assert!(name.starts_with(SESSION_PREFIX));
        assert!(name.contains("My_Project"));
        assert!(name.contains("abc123de"));
    }

    #[test]
    fn test_detect_claude_status() {
        assert_eq!(detect_claude_status("thinking about your request..."), Status::Running);
        assert_eq!(detect_claude_status("claude> "), Status::Waiting);
        assert_eq!(detect_claude_status("error: something went wrong"), Status::Error);
        assert_eq!(detect_claude_status("completed the task"), Status::Idle);
    }
}
