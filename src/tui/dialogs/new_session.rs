//! New session dialog

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::DialogResult;
use crate::tui::styles::Theme;

const TOOL_OPTIONS: [&str; 2] = ["claude", "opencode"];

pub struct NewSessionData {
    pub title: String,
    pub path: String,
    pub group: String,
    pub tool: String,
}

pub struct NewSessionDialog {
    title: String,
    path: String,
    group: String,
    tool_index: usize,
    focused_field: usize,
}

impl NewSessionDialog {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        Self {
            title: String::new(),
            path: current_dir,
            group: String::new(),
            tool_index: 0,
            focused_field: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult<NewSessionData> {
        match key.code {
            KeyCode::Esc => DialogResult::Cancel,
            KeyCode::Enter => {
                if self.title.is_empty() {
                    self.title = std::path::Path::new(&self.path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "untitled".to_string());
                }
                DialogResult::Submit(NewSessionData {
                    title: self.title.clone(),
                    path: self.path.clone(),
                    group: self.group.clone(),
                    tool: TOOL_OPTIONS[self.tool_index].to_string(),
                })
            }
            KeyCode::Tab => {
                self.focused_field = (self.focused_field + 1) % 4;
                DialogResult::Continue
            }
            KeyCode::BackTab => {
                self.focused_field = if self.focused_field == 0 {
                    3
                } else {
                    self.focused_field - 1
                };
                DialogResult::Continue
            }
            KeyCode::Left | KeyCode::Right if self.focused_field == 3 => {
                self.tool_index = 1 - self.tool_index;
                DialogResult::Continue
            }
            KeyCode::Char(' ') if self.focused_field == 3 => {
                self.tool_index = 1 - self.tool_index;
                DialogResult::Continue
            }
            KeyCode::Backspace => {
                if self.focused_field != 3 {
                    self.current_field_mut().pop();
                }
                DialogResult::Continue
            }
            KeyCode::Char(c) => {
                if self.focused_field != 3 {
                    self.current_field_mut().push(c);
                }
                DialogResult::Continue
            }
            _ => DialogResult::Continue,
        }
    }

    fn current_field_mut(&mut self) -> &mut String {
        match self.focused_field {
            0 => &mut self.title,
            1 => &mut self.path,
            2 => &mut self.group,
            _ => &mut self.title,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_width = 60;
        let dialog_height = 14;
        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect {
            x,
            y,
            width: dialog_width.min(area.width),
            height: dialog_height.min(area.height),
        };

        let clear = Clear;
        frame.render_widget(clear, dialog_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(" New Session ")
            .title_style(Style::default().fg(theme.title).bold());

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(inner);

        let text_fields = [
            ("Title:", &self.title),
            ("Path:", &self.path),
            ("Group:", &self.group),
        ];

        for (idx, (label, value)) in text_fields.iter().enumerate() {
            let is_focused = idx == self.focused_field;
            let style = if is_focused {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text)
            };

            let display_value = if value.is_empty() && idx == 0 {
                "(directory name)"
            } else {
                value.as_str()
            };

            let text = format!("{} {}", label, display_value);
            let cursor = if is_focused { "█" } else { "" };
            let line = Line::from(vec![
                Span::styled(text, style),
                Span::styled(cursor, Style::default().fg(theme.accent)),
            ]);

            frame.render_widget(Paragraph::new(line), chunks[idx]);
        }

        let is_tool_focused = self.focused_field == 3;
        let tool_style = if is_tool_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.text)
        };

        let claude_style = if self.tool_index == 0 {
            Style::default().fg(theme.accent).bold()
        } else {
            Style::default().fg(theme.dimmed)
        };
        let opencode_style = if self.tool_index == 1 {
            Style::default().fg(theme.accent).bold()
        } else {
            Style::default().fg(theme.dimmed)
        };

        let tool_line = Line::from(vec![
            Span::styled("Tool:  ", tool_style),
            Span::styled(if self.tool_index == 0 { "● " } else { "○ " }, claude_style),
            Span::styled("claude", claude_style),
            Span::raw("   "),
            Span::styled(
                if self.tool_index == 1 { "● " } else { "○ " },
                opencode_style,
            ),
            Span::styled("opencode", opencode_style),
        ]);
        frame.render_widget(Paragraph::new(tool_line), chunks[3]);

        let hint = Line::from(vec![
            Span::styled("Tab", Style::default().fg(theme.hint)),
            Span::raw(" next  "),
            Span::styled("←/→/Space", Style::default().fg(theme.hint)),
            Span::raw(" toggle tool  "),
            Span::styled("Enter", Style::default().fg(theme.hint)),
            Span::raw(" create  "),
            Span::styled("Esc", Style::default().fg(theme.hint)),
            Span::raw(" cancel"),
        ]);
        frame.render_widget(Paragraph::new(hint), chunks[4]);
    }
}

impl Default for NewSessionDialog {
    fn default() -> Self {
        Self::new()
    }
}
