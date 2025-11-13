use std::{
    cmp::{max, min}, collections::HashMap, io, path::PathBuf
};

use ratatui::{
    DefaultTerminal, Frame, crossterm::event, layout::{self, Constraint, Layout, Rect}, prelude, style::{Color, Modifier, Style}, widgets::{Paragraph, Widget}
};

use crate::app::{
    clipboard::Clipboard, error_msg::ErrorMessage, logic::calc::{CellType, Grid}, mode::Mode, screen::ScreenSpace
};

pub struct App {
    pub exit: bool,
    pub grid: Grid,
    pub mode: Mode,
    pub file: Option<PathBuf>,
    pub error_msg: ErrorMessage,
    pub vars: HashMap<String, String>,
    pub screen: ScreenSpace,
    // this could probably be a normal array
    pub marks: HashMap<char, (usize, usize)>,
    pub clipboard: Clipboard,
}

impl Widget for &App {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        let (x_max, y_max) = self.screen.how_many_cells_fit_in(&area, &self.vars);

        let is_selected = |x: usize, y: usize| -> bool {
            if let Mode::Visual((mut x1, mut y1)) = self.mode {
                let (mut x2, mut y2) = self.grid.cursor();
                x1 += 1;
                y1 += 1;
                x2 += 1;
                y2 += 1;

                // in-between the Xs
                if (x >= x1 && x <= x2) || (x >= x2 && x <= x1) {
                    // in-between the Ys
                    if (y >= y1 && y <= y2) || (y >= y2 && y <= y1) {
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

                let cell_width = self.screen.get_cell_width(&self.vars) as u16;
                let cell_height = self.screen.get_cell_height(&self.vars) as u16;

                // Minus 1 because of header cells,
                // the grid is shifted over (1,1), so if you need
                // to index the grid, these are you values.
                let mut x_idx: usize = 0;
                let mut y_idx: usize = 0;
                if x != 0 {
                    x_idx = x as usize - 1 + self.screen.scroll_x();
                }
                if y != 0 {
                    y_idx = y as usize - 1 + self.screen.scroll_y();
                }

                if is_selected(x.into(), y.into()) {
                    style = style.fg(Color::LightMagenta).bg(Color::Blue);
                }

                const ORANGE1: Color = Color::Rgb(200, 160, 0);
                const ORANGE2: Color = Color::Rgb(180, 130, 0);

                let mut should_render = true;
                let mut suggest_upper_bound = None;

                match (x == 0, y == 0) {
                    // 0,0 vi mode
                    (true, true) => {
                        display = self.mode.to_string();
                        style = self.mode.get_style();
                    }
                    // row names
                    (true, false) => {
                        display = y_idx.to_string();

                        let bg = if y_idx == self.grid.cursor().1 {
                            Color::DarkGray
                        } else if y_idx % 2 == 0 {
                            ORANGE1
                        } else {
                            ORANGE2
                        };
                        style = Style::new().fg(Color::White).bg(bg);
                    }
                    // column names
                    (false, true) => {
                        display = Grid::num_to_char(x_idx);

                        let bg = if x_idx == self.grid.cursor().0 {
                            Color::DarkGray
                        } else if x_idx % 2 == 0 {
                            ORANGE1
                        } else {
                            ORANGE2
                        };

                        style = Style::new().fg(Color::White).bg(bg)
                    }
                    // grid squares
                    (false, false) => {
                        match self.grid.get_cell_raw(x_idx, y_idx) {
                            Some(cell) => {
                                match cell {
                                    CellType::Number(c) => display = c.to_string(),
                                    CellType::String(s) => display = s.to_owned(),
                                    CellType::Equation(e) => {
                                        match self.grid.evaluate(e) {
                                            Ok(val) => {
                                                display = val.to_string();
                                                style = Style::new()
                                                    .underline_color(Color::DarkGray)
                                                    .add_modifier(Modifier::UNDERLINED);
                                            }
                                            Err(err) => {
                                                // the formula is broken
                                                display = err.to_owned();
                                                style = Style::new()
                                                    .fg(Color::Red)
                                                    .underline_color(Color::Red)
                                                    .add_modifier(Modifier::UNDERLINED)
                                            }
                                        }
                                    }
                                }

                                suggest_upper_bound = Some(display.len() as u16);
                                // check for cells to the right, see if we should truncate the cell width
                                for i in 1..(display.len() as f32 / cell_width as f32).ceil() as usize {
                                    if let Some(_) = self.grid.get_cell_raw(x_idx + i, y_idx) {
                                        suggest_upper_bound = Some(cell_width * i as u16);
                                        break;
                                    }
                                }
                            }
                            None => should_render = false,
                        }
                        if (x_idx, y_idx) == self.grid.cursor() {
                            should_render = true;
                            style = Style::new().fg(Color::Black).bg(Color::White);
                            // modify the style of the cell you are editing
                            if let Mode::Insert(_) = self.mode {
                                style = style.add_modifier(Modifier::ITALIC).add_modifier(Modifier::BOLD);
                            }
                        }
                    }
                }
                if should_render {
                    let x_off = area.x + (x * cell_width);
                    let y_off = area.y + (y * cell_height);

                    let area = if let Some(suggestion) = suggest_upper_bound {
                        let max_available_width = area.width - x_off;
                        // draw the biggest cell possible, without going OOB off the screen
                        let width = min(max_available_width, suggestion as u16);
                        // Don't draw too small tho, we want full-sized cells, minium
                        let width = max(cell_width, width);

                        Rect::new(x_off, y_off, width, cell_height)
                    } else {
                        Rect::new(x_off, y_off, cell_width, cell_height)
                    };

                    Paragraph::new(display).style(style).render(area, buf);
                }
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
            error_msg: ErrorMessage::none(),
            vars: HashMap::new(),
            screen: ScreenSpace::new(),
            marks: HashMap::new(),
            clipboard: Clipboard::new(),
        }
    }

    pub fn new_with_file(file: impl Into<PathBuf> + Clone) -> std::io::Result<Self> {
        let mut app = Self::new();
        app.file = Some(file.clone().into());
        app.grid = Grid::new_from_file(file.into())?;
        Ok(app)
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

        let len = match &self.mode {
            Mode::Insert(edit) | Mode::Command(edit) | Mode::Chord(edit) => edit.len(),
            Mode::Normal => {
                let (x, y) = self.grid.cursor();
                let cell = self.grid.get_cell_raw(x, y).as_ref().map(|f| f.to_string().len()).unwrap_or_default();
                cell
            }
            Mode::Visual(_) => 0,
        };
        // min 20 chars, expand if needed
        let len = max(len as u16 + 1, 20);

        let bottom_split = Layout::default()
            .direction(layout::Direction::Horizontal)
            .constraints([Constraint::Length(len), Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(cmd_line);

        let cmd_line_left = bottom_split[0];
        let cmd_line_right = bottom_split[1];
        let cmd_line_debug = bottom_split[2];

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
                    let (x, y) = self.grid.cursor();
                    let cell = self.grid.get_cell_raw(x, y).as_ref().map(|f| f.to_string()).unwrap_or_default();
                    cell
                }),
                cmd_line_left,
            ),
            Mode::Visual(_) => {}
        }

        frame.render_widget(self, body);
        frame.render_widget(&self.error_msg, cmd_line_right);
        #[cfg(debug_assertions)]
        frame.render_widget(
            Paragraph::new(format!(
                "x/w y/h: cursor{:?} scroll({}, {}) cell({}, {}) screen({}, {}) len{len}",
                self.grid.cursor(),
                self.screen.scroll_x(),
                self.screen.scroll_y(),
                self.screen.get_cell_width(&self.vars),
                self.screen.get_cell_height(&self.vars),
                body.width,
                body.height,
            )),
            cmd_line_debug,
        );
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
                        let v = editor.as_string();

                        // try to insert as a float
                        if let Ok(v) = v.parse::<f64>() {
                            self.grid.set_cell_raw(self.grid.cursor(), Some(v));
                        } else {
                            // if you can't, then insert as a string
                            if !v.is_empty() {
                                self.grid.set_cell_raw(self.grid.cursor(), Some(v));
                            } else {
                                self.grid.set_cell_raw::<CellType>(self.grid.cursor(), None);
                            }
                        }

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
            Mode::Normal => match event::read()? {
                event::Event::Key(key_event) => match key_event.code {
                    event::KeyCode::F(_) => todo!(),
                    event::KeyCode::Char(c) => {
                        Mode::process_key(self, c);
                    }
                    _ => {}
                },
                _ => {}
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

        // make sure cursor is inside window
        self.screen.scroll_based_on_cursor_location(self.grid.cursor(), &self.vars);

        Ok(())
    }
}

#[test]
fn test_quit_cmd() {
    let mut app = App::new();
    assert!(!app.exit);

    app.mode = Mode::Command(crate::app::mode::Chord::from(":q".to_string()));
    Mode::process_cmd(&mut app);

    assert!(app.exit);
}
