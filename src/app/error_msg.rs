use std::time::Instant;

use ratatui::{layout::Rect, prelude, style::{Color, Style}, widgets::{Paragraph, Widget}};

pub enum MsgType {
    Error,
    Info
}

pub struct StatusMessage {
    start: Instant,
    msg_type: MsgType,
    msg: Option<String>,
}

impl StatusMessage {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            msg: Some(msg.into()),
            start: Instant::now(),
            msg_type: MsgType::Info,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            msg: Some(msg.into()),
            start: Instant::now(),
            msg_type: MsgType::Error,
        }
    }

    pub fn none() -> Self {
        Self {
            start: Instant::now(),
            msg: None,
            msg_type: MsgType::Info,
        }
    }
}

impl Widget for &StatusMessage {
    fn render(self, area: Rect, buf: &mut prelude::Buffer) {
        // The screen doesn't refresh at a fixed fps like a normal GUI,
        // so if the user isn't moving around the timeout will *happen* but
        // won't be visualized until they move
        let msg = if self.start.elapsed().as_secs() > 3 {
            String::new()
        } else {
            self.msg.clone().unwrap_or_default()
        };

        let style = match self.msg_type {
            MsgType::Error => Style::new().fg(Color::Red),
            MsgType::Info => Style::new().fg(Color::LightGreen),
        };

        Paragraph::new(msg).style(style).render(area, buf);
    }
}
