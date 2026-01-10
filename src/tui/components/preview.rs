//! Preview panel component

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::session::Instance;
use crate::tui::styles::Theme;

pub struct Preview;

impl Preview {
    pub fn render(frame: &mut Frame, area: Rect, instance: &Instance, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Info section
                Constraint::Min(1),    // Output section
            ])
            .split(area);

        Self::render_info(frame, chunks[0], instance, theme);
        Self::render_output(frame, chunks[1], instance, theme);
    }

    fn render_info(frame: &mut Frame, area: Rect, instance: &Instance, theme: &Theme) {
        let info_lines = vec![
            Line::from(vec![
                Span::styled("Title:   ", Style::default().fg(theme.dimmed)),
                Span::styled(&instance.title, Style::default().fg(theme.text).bold()),
            ]),
            Line::from(vec![
                Span::styled("Path:    ", Style::default().fg(theme.dimmed)),
                Span::styled(
                    shorten_path(&instance.project_path),
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(vec![
                Span::styled("Tool:    ", Style::default().fg(theme.dimmed)),
                Span::styled(&instance.tool, Style::default().fg(theme.accent)),
            ]),
            Line::from(vec![
                Span::styled("Status:  ", Style::default().fg(theme.dimmed)),
                Span::styled(
                    format!("{:?}", instance.status),
                    Style::default().fg(match instance.status {
                        crate::session::Status::Running => theme.running,
                        crate::session::Status::Waiting => theme.waiting,
                        crate::session::Status::Idle => theme.idle,
                        crate::session::Status::Error => theme.error,
                        crate::session::Status::Starting => theme.dimmed,
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Group:   ", Style::default().fg(theme.dimmed)),
                Span::styled(
                    if instance.group_path.is_empty() {
                        "(none)"
                    } else {
                        &instance.group_path
                    },
                    Style::default().fg(theme.group),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(info_lines);
        frame.render_widget(paragraph, area);
    }

    fn render_output(frame: &mut Frame, area: Rect, instance: &Instance, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.border))
            .title(" Output ")
            .title_style(Style::default().fg(theme.dimmed));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // #region agent log
        use std::io::Write;
        let log_path = "/Users/nbrake/scm/agent-of-empires/.cursor/debug.log";
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
        {
            let _ = writeln!(
                f,
                r#"{{"hypothesisId":"E","location":"preview.rs:render_output","message":"render_output dimensions","data":{{"area_width":{},"area_height":{},"inner_width":{},"inner_height":{},"instance_id":"{}"}},"timestamp":{}}}"#,
                area.width,
                area.height,
                inner.width,
                inner.height,
                instance.id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            );
        }
        // #endregion

        if let Some(error) = &instance.last_error {
            let error_lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    "Error:",
                    Style::default().fg(theme.error).bold(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    error.as_str(),
                    Style::default().fg(theme.error),
                )),
            ];
            let paragraph = Paragraph::new(error_lines).wrap(Wrap { trim: false });
            frame.render_widget(paragraph, inner);
            return;
        }

        let output = instance
            .capture_output_with_size(inner.height as usize, inner.width, inner.height)
            .unwrap_or_default();

        if output.is_empty() {
            let hint = Paragraph::new("No output available")
                .style(Style::default().fg(theme.dimmed))
                .alignment(Alignment::Center);
            frame.render_widget(hint, inner);
        } else {
            let output_lines: Vec<Line> = output
                .lines()
                .map(|line| Line::from(Span::raw(line)))
                .collect();

            let line_count = output_lines.len();
            let visible_height = inner.height as usize;

            // Scroll to show the bottom of the content
            let scroll_offset = if line_count > visible_height {
                (line_count - visible_height) as u16
            } else {
                0
            };

            let paragraph = Paragraph::new(output_lines)
                .style(Style::default().fg(theme.text))
                .scroll((scroll_offset, 0));

            frame.render_widget(paragraph, inner);
        }
    }
}

fn shorten_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Some(home_str) = home.to_str() {
            if let Some(stripped) = path.strip_prefix(home_str) {
                return format!("~{}", stripped);
            }
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shorten_path_with_home() {
        if let Some(home) = dirs::home_dir() {
            if let Some(home_str) = home.to_str() {
                let path = format!("{}/projects/myapp", home_str);
                let shortened = shorten_path(&path);
                assert_eq!(shortened, "~/projects/myapp");
            }
        }
    }

    #[test]
    fn test_shorten_path_without_home_prefix() {
        let path = "/tmp/some/path";
        let shortened = shorten_path(path);
        assert_eq!(shortened, "/tmp/some/path");
    }

    #[test]
    fn test_shorten_path_exact_home() {
        if let Some(home) = dirs::home_dir() {
            if let Some(home_str) = home.to_str() {
                let shortened = shorten_path(home_str);
                assert_eq!(shortened, "~");
            }
        }
    }

    #[test]
    fn test_shorten_path_relative() {
        let path = "relative/path";
        let shortened = shorten_path(path);
        assert_eq!(shortened, "relative/path");
    }

    #[test]
    fn test_shorten_path_empty() {
        let path = "";
        let shortened = shorten_path(path);
        assert_eq!(shortened, "");
    }

    #[test]
    fn test_shorten_path_similar_prefix_not_home() {
        if let Some(home) = dirs::home_dir() {
            if let Some(home_str) = home.to_str() {
                let path = format!("{}extra/not/home", home_str);
                let shortened = shorten_path(&path);
                assert_eq!(shortened, format!("~extra/not/home"));
            }
        }
    }

    #[test]
    fn test_shorten_path_preserves_trailing_slash() {
        if let Some(home) = dirs::home_dir() {
            if let Some(home_str) = home.to_str() {
                let path = format!("{}/projects/", home_str);
                let shortened = shorten_path(&path);
                assert_eq!(shortened, "~/projects/");
            }
        }
    }
}
