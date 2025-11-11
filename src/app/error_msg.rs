use std::time::Instant;

use ratatui::{layout::Rect, prelude, style::{Color, Style}, widgets::{Paragraph, Widget}};

pub struct ErrorMessage {
    start: Instant,
    error_msg: Option<String>,
}

impl ErrorMessage {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            error_msg: Some(msg.into()),
            start: Instant::now(),
        }
    }

    pub fn none() -> Self {
        Self {
            start: Instant::now(),
            error_msg: None,
        }
    }
}

impl Widget for &ErrorMessage {
    fn render(self, area: Rect, buf: &mut prelude::Buffer) {
        // The screen doesn't refresh at a fixed fps like a normal GUI,
        // so if the user isn't moving around the timeout will *happen* but
        // won't be visualized until they move
        let msg = if self.start.elapsed().as_secs() > 3 {
            String::new()
        } else {
            self.error_msg.clone().unwrap_or(String::new())
        };

        Paragraph::new(msg).style(Style::new().fg(Color::Red)).render(area, buf);
    }
}