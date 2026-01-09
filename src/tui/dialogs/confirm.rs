//! Confirmation dialog

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::DialogResult;
use crate::tui::styles::Theme;

pub struct ConfirmDialog {
    title: String,
    message: String,
    action: String,
    selected: bool, // true = Yes, false = No
}

impl ConfirmDialog {
    pub fn new(title: &str, message: &str, action: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            action: action.to_string(),
            selected: false,
        }
    }

    pub fn action(&self) -> &str {
        &self.action
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => DialogResult::Cancel,
            KeyCode::Enter => {
                if self.selected {
                    DialogResult::Submit(())
                } else {
                    DialogResult::Cancel
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => DialogResult::Submit(()),
            KeyCode::Left | KeyCode::Char('h') => {
                self.selected = true;
                DialogResult::Continue
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected = false;
                DialogResult::Continue
            }
            KeyCode::Tab => {
                self.selected = !self.selected;
                DialogResult::Continue
            }
            _ => DialogResult::Continue,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_width = 50;
        let dialog_height = 8;
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
            .border_style(Style::default().fg(theme.error))
            .title(format!(" {} ", self.title))
            .title_style(Style::default().fg(theme.error).bold());

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner);

        // Message
        let message = Paragraph::new(&*self.message)
            .style(Style::default().fg(theme.text))
            .wrap(Wrap { trim: true });
        frame.render_widget(message, chunks[0]);

        // Buttons
        let yes_style = if self.selected {
            Style::default().fg(theme.error).bold()
        } else {
            Style::default().fg(theme.dimmed)
        };
        let no_style = if !self.selected {
            Style::default().fg(theme.running).bold()
        } else {
            Style::default().fg(theme.dimmed)
        };

        let buttons = Line::from(vec![
            Span::raw("  "),
            Span::styled("[Yes]", yes_style),
            Span::raw("    "),
            Span::styled("[No]", no_style),
        ]);

        frame.render_widget(
            Paragraph::new(buttons).alignment(Alignment::Center),
            chunks[1],
        );
    }
}
