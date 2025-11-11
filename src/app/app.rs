use std::{io, path::PathBuf};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event,
    layout::{self, Constraint, Layout, Rect},
    prelude,
    style::{Color, Modifier, Style},
    widgets::{Paragraph, Widget},
};

use crate::app::{
    calc::{Grid, LEN},
    mode::Mode,
};

pub struct App {
    pub exit: bool,
    pub grid: Grid,
    pub mode: Mode,
    pub file: Option<PathBuf>,
}

impl Widget for &App {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        let len = LEN as u16;

        let cell_height = 1;
        let cell_length = 10;

        let x_max = if area.width / cell_length > len { len - 1 } else { area.width / cell_length };
        let y_max = if area.height / cell_height > len { len - 1 } else { area.height / cell_height };

        let is_selected = |x: usize, y: usize| -> bool {
            if let Mode::Visual((mut x1, mut y1)) = self.mode {
                let (mut x2, mut y2) = self.grid.selected_cell;
                x1 += 1;
                y1 += 1;
                x2 += 1;
                y2 += 1;

                if (x >= x1 && x <= x2) || (x >= x2 && x <= x1) {
                    // in-between the Xs
                    if (y >= y1 && y <= y2) || (y >= y2 && y <= y1) {
                        // in-between the Ys
                        return true;
                    }
                }
            }
            false
        };

        for x in 0..x_max {
            for y in 0..y_max {
                let mut display = String::new();
                let mut style = Style::new().fg(Color::White);

                // Minus 1 because of header cells,
                // the grid is shifted over (1,1), so if you need
                // to index the grid, these are you values.
                let mut x_idx: usize = 0;
                let mut y_idx: usize = 0;
                if y != 0 {
                    y_idx = y as usize - 1;
                }
                if x != 0 {
                    x_idx = x as usize - 1;
                }

                if is_selected(x.into(), y.into()) {
                    style = style.fg(Color::LightMagenta).bg(Color::Blue);
                }

                const ORANGE1: Color = Color::Rgb(200, 160, 0);
                const ORANGE2: Color = Color::Rgb(180, 130, 0);

                match (x == 0, y == 0) {
                    (true, true) => {
                        let (x, y) = self.grid.selected_cell;
                        let c = Grid::num_to_char(x);
                        display = format!("{y}{c}",);
                        style = Style::new().fg(Color::Green).bg(Color::Black);
                    }
                    (true, false) => {
                        // row names
                        display = y_idx.to_string();

                        let bg = if y_idx == self.grid.selected_cell.1 {
                            Color::DarkGray
                        } else if y_idx % 2 == 0 {
                            ORANGE1
                        } else {
                            ORANGE2
                        };
                        style = Style::new().fg(Color::White).bg(bg);
                    }
                    (false, true) => {
                        // column names
                        display = Grid::num_to_char(x_idx);

                        let bg = if x_idx == self.grid.selected_cell.0 {
                            Color::DarkGray
                        } else if x_idx % 2 == 0 {
                            ORANGE1
                        } else {
                            ORANGE2
                        };

                        style = Style::new().fg(Color::White).bg(bg)
                    }
                    (false, false) => {
                        if let Some(cell) = self.grid.get_cell_raw(x_idx, y_idx) {
                            display = cell.as_raw_string();

                            if cell.can_be_number() {
                                if let Some(val) = self.grid.evaluate(&cell.as_raw_string()) {
                                    display = val.to_string();
                                    style = Style::new()
                                        .underline_color(Color::DarkGray)
                                        .add_modifier(Modifier::UNDERLINED);
                                } else {
                                    // broken formulas
                                    if cell.is_equation() {
                                        style =
                                            Style::new().underline_color(Color::Red).add_modifier(Modifier::UNDERLINED)
                                    }
                                }
                            }
                        }
                        if (x_idx, y_idx) == self.grid.selected_cell {
                            style = Style::new().fg(Color::Black).bg(Color::White);
                            // modify the style of the cell you are editing
                            if let Mode::Insert(_) = self.mode {
                                style = style.add_modifier(Modifier::ITALIC).add_modifier(Modifier::BOLD);
                            }
                        }
                    }
                }

                let area = Rect::new(area.x + (x * cell_length), area.y + (y * cell_height), cell_length, cell_height);

                Paragraph::new(display).style(style).render(area, buf);
            }
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            exit: false,
            grid: Grid::new(),
            mode: Mode::Normal,
            file: None,
        }
    }

    pub fn run(&mut self, mut term: DefaultTerminal) -> Result<(), std::io::Error> {
        while !self.exit {
            term.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(frame.area());

        let cmd_line = layout[0];
        let body = layout[1];

        let bottom_split = Layout::default()
            .direction(layout::Direction::Horizontal)
            .constraints([Constraint::Min(30), Constraint::Min(100)])
            .split(cmd_line);

        let cmd_line_left = bottom_split[0];
        let cmd_line_right = bottom_split[1];

        match &self.mode {
            Mode::Insert(editor) => {
                frame.render_widget(editor, cmd_line_left);
            }
            Mode::Command(editor) => {
                frame.render_widget(editor, cmd_line_left);
            }
            Mode::Chord(chord) => frame.render_widget(chord, cmd_line_left),
            Mode::Normal => frame.render_widget(
                Paragraph::new({
                    let (x, y) = self.grid.selected_cell;
                    let cell = self.grid.get_cell_raw(x, y).as_ref().map(|f| f.as_raw_string()).unwrap_or_default();
                    cell
                }),
                cmd_line_left,
            ),
            Mode::Visual(_) => {}
        }

        frame.render_widget(self, body);
        frame.render_widget(&self.mode, cmd_line_right);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match &mut self.mode {
            Mode::Chord(chord) => match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => self.mode = Mode::Normal,
                    event::KeyCode::Char(c) => {
                        Mode::process_key(self, c);
                    }
                    event::KeyCode::Backspace => {
                        chord.backspace();
                    }
                    _ => {}
                },
                _ => {}
            },
            Mode::Insert(editor) => match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => {
                        // just cancel the operation
                        self.mode = Mode::Normal;
                    }
                    event::KeyCode::Enter => {
                        let v = editor.buf.trim().to_string();

                        if let Ok(v) = v.parse::<f64>() {
                            self.grid.set_cell_raw(editor.location, v);
                        } else {
                            self.grid.set_cell_raw(editor.location, v);
                        }

                        self.mode = Mode::Normal;
                    }
                    event::KeyCode::Backspace => {
                        editor.buf.pop();
                    }
                    event::KeyCode::Char(c) => {
                        editor.buf += &c.to_string();
                    }
                    _ => {}
                },
                _ => {}
            },
            Mode::Normal => match event::read()? {
                event::Event::Key(key_event) => match key_event.code {
                    event::KeyCode::F(_) => todo!(),
                    event::KeyCode::Char(c) => {
                        Mode::process_key(self, c);
                    }
                    _ => todo!(),
                },
                _ => todo!(),
            },
            Mode::Visual(_start_pos) => {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        event::KeyCode::Char(c) => {
                            Mode::process_key(self, c);
                        }
                        event::KeyCode::Esc => self.mode = Mode::Normal,
                        _ => {}
                    }
                }
            }
            Mode::Command(editor) => match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => {
                        // just cancel the operation
                        self.mode = Mode::Normal;
                    }
                    event::KeyCode::Enter => {
                        Mode::process_cmd(self);
                        self.mode = Mode::Normal;
                    }
                    event::KeyCode::Backspace => {
                        editor.backspace();
                    }
                    event::KeyCode::Char(c) => {
                        editor.add_char(c);
                    }
                    _ => {}
                },
                _ => {}
            },
        }

        Ok(())
    }
}
