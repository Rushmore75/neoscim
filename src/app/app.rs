use std::{
    cmp::{max, min},
    collections::HashMap,
    fs, io,
    path::PathBuf,
    time::SystemTime,
};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event,
    layout::{self, Constraint, Layout, Rect},
    prelude,
    style::{Color, Modifier, Style},
    widgets::{Paragraph, Widget},
};

use crate::app::{
    clipboard::Clipboard,
    error_msg::StatusMessage,
    logic::{
        calc::{Grid, get_header_size},
        cell::CellType,
    },
    mode::Mode,
    screen::ScreenSpace,
};

pub struct App {
    pub exit: bool,
    pub grid: Grid,
    pub mode: Mode,
    pub file: Option<PathBuf>,
    file_modified_date: SystemTime,
    pub msg: StatusMessage,
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
            if let Mode::Visual((mut x1, mut y1)) | Mode::VisualCmd((mut x1, mut y1), _) = self.mode {
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
                let mut style = Style::new();

                // Custom width for the header of each row
                let row_header_width = get_header_size() as u16;
                // ^^ Feels like it oculd be static but evaluating string lens doesn't work at
                // compile time. Thus cannot be static.
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

                const ORANGE1: Color = Color::Rgb(200, 160, 0);
                const ORANGE2: Color = Color::Rgb(180, 130, 0);

                let mut should_render = true;
                let mut suggest_upper_bound = None;

                /// Center the text "99  " -> " 99 "
                fn center_text(text: &str, avaliable_space: i32) -> String {
                    let margin = avaliable_space - text.len() as i32;
                    let margin = margin / 2;
                    let l_margin = (0..margin).into_iter().map(|_| ' ').collect::<String>();
                    let r_margin = (0..(margin - (l_margin.len() as i32))).into_iter().map(|_| ' ').collect::<String>();
                    format!("{l_margin}{text}{r_margin}")
                }

                match (x == 0, y == 0) {
                    // 0,0 vi mode
                    (true, true) => {
                        display = self.mode.to_string();
                        style = self.mode.get_style();
                    }
                    // row names
                    (true, false) => {
                        display = center_text(&y_idx.to_string(), row_header_width as i32);

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
                        display = center_text(&Grid::num_to_char(x_idx), cell_width as i32);

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
                                // Render in different colors based on type of contents
                                match cell {
                                    CellType::Number(c) => display = c.to_string(),
                                    CellType::String(s) => {
                                        display = s.to_owned();
                                        style = Style::new().fg(Color::LightMagenta)
                                    }
                                    CellType::Equation(e) => {
                                        match self.grid.evaluate(e) {
                                            Ok(val) => {
                                                display = val.to_string();
                                                style = Style::new()
                                                    .fg(Color::White)
                                                    // TODO This breaks dumb terminals like the windows
                                                    // terminal
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

                                // Allow for text in one cell to visually overflow into empty cells
                                suggest_upper_bound = Some(display.len() as u16);
                                // check for cells to the right, see if we should truncate the cell width
                                for i in 1..(display.len() as f32 / cell_width as f32).ceil() as usize {
                                    if let Some(_) = self.grid.get_cell_raw(x_idx + i, y_idx) {
                                        suggest_upper_bound = Some(cell_width * i as u16);
                                        break;
                                    }
                                }
                                if let Some(bound) = suggest_upper_bound {
                                    let bound = bound as usize;
                                    if bound < display.len() {
                                        display.truncate(bound - 2);
                                        display.push('…');
                                    }
                                }
                            }
                            // Don't render blank cells
                            None => should_render = false,
                        }

                        if is_selected(x.into(), y.into()) {
                            style = style.bg(Color::Blue);
                            // Make it so that cells render when selected. This fixes issue #32
                            should_render = true;
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
                    let mut x_off = area.x + (x * cell_width);
                    let y_off = area.y + (y * cell_height);

                    // Adjust for the fact that the first column
                    // is smaller, since it is just headers
                    if x > 0 {
                        x_off = x_off - (cell_width - row_header_width);
                    }

                    // If this is the row header column
                    let area = if x == 0 && y != 0 {
                        Rect::new(x_off, y_off, row_header_width, cell_height)
                    } else if let Some(suggestion) = suggest_upper_bound {
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
            msg: StatusMessage::none(),
            vars: HashMap::new(),
            screen: ScreenSpace::new(),
            marks: HashMap::new(),
            clipboard: Clipboard::new(),
            file_modified_date: SystemTime::now(),
        }
    }

    pub fn new_with_file(file: impl Into<PathBuf> + Clone) -> std::io::Result<Self> {
        let mut app = Self::new();
        app.file = Some(file.clone().into());

        let mut file = fs::OpenOptions::new().read(true).open(file.into())?;
        let metadata = file.metadata()?;
        // Not all systems support this, apparently.
        if let Ok(time) = metadata.modified() {
            app.file_modified_date = time;
        } else {
            // Default is to just assume it was modified when we opened it.
        }

        app.grid = Grid::new_from_file(&mut file)?;
        Ok(app)
    }

    pub fn run(&mut self, mut term: DefaultTerminal) -> Result<(), std::io::Error> {
        while !self.exit {
            term.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn file_name_display(&self) -> String {
        let file_name_status = {
            let mut file_name = "[No Name]";
            let mut icon = "";
            if let Some(file) = &self.file {
                if let Some(f) = file.file_name() {
                    if let Some(f) = f.to_str() {
                        file_name = f;
                    }
                }
            }
            if self.grid.needs_to_be_saved() {
                icon = "[+]";
            }
            format!("{file_name}{icon}")
        };
        file_name_status
    }

    fn draw(&self, frame: &mut Frame) {
        let (x, y) = self.grid.cursor();
        let current_cell = self.grid.get_cell_raw(x, y);
        let len = self.mode.chars_to_display(current_cell);
        let file_name_status = self.file_name_display();

        // layout
        // ======================================================
        let layout = Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(frame.area());
        let cmd_line = layout[0];
        let body = layout[1];

        let cmd_line_split = Layout::default()
            .direction(layout::Direction::Horizontal)
            .constraints([
                Constraint::Length(len),
                Constraint::Length(file_name_status.len() as u16 + 1),
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(cmd_line);

        let cmd_line_left = cmd_line_split[0];
        let cmd_line_status = cmd_line_split[1];
        let cmd_line_right = cmd_line_split[2];

        #[cfg(debug_assertions)]
        let cmd_line_debug = cmd_line_split[3];
        // ======================================================

        self.mode.render(frame, cmd_line_left, current_cell);

        frame.render_widget(self, body);
        frame.render_widget(&self.msg, cmd_line_right);
        frame.render_widget(Paragraph::new(file_name_status), cmd_line_status);

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
            Mode::VisualCmd(pos, chord) => match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => self.mode = Mode::Visual(*pos),
                    event::KeyCode::Backspace => chord.backspace(),
                    event::KeyCode::Char(c) => chord.add_char(c),
                    event::KeyCode::Enter => {
                        // tmp is to get around reference issues.
                        let tmp = pos.clone();
                        Mode::process_cmd(self);
                        self.mode = Mode::Visual(tmp)
                    }
                    _ => {}
                },
                _ => {}
            },
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

                        let cursor = self.grid.cursor();
                        self.grid.transact_on_grid(|grid| {
                            // try to insert as a float
                            if let Ok(v) = v.parse::<f64>() {
                                grid.set_cell_raw(cursor, Some(v));
                            } else {
                                // if you can't, then insert as a string
                                if !v.is_empty() {
                                    grid.set_cell_raw(cursor, Some(v.to_owned()));
                                } else {
                                    grid.set_cell_raw::<CellType>(cursor, None);
                                }
                            }
                        });

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
                    event::KeyCode::F(n) => {},
                    event::KeyCode::Char(c) => Mode::process_key(self, c),
                    // Pretend that the arrow keys are vim movement keys
                    event::KeyCode::Left => Mode::process_key(self, 'h'),
                    event::KeyCode::Right => Mode::process_key(self, 'l'),
                    event::KeyCode::Up => Mode::process_key(self, 'k'),
                    event::KeyCode::Down => Mode::process_key(self, 'j'),
                    // Getting ctrl to work isn't going will right now. Use page keys for the time being.
                    event::KeyCode::PageUp => self.grid.redo(),
                    event::KeyCode::PageDown => self.grid.undo(),
                    event::KeyCode::Modifier(modifier_key_code) => {
                        if let event::ModifierKeyCode::LeftControl | event::ModifierKeyCode::RightControl = modifier_key_code {
                            // TODO my terminal (alacritty) isn't showing me ctrl presses. I know
                            // that they work tho, since ctrl+r works here in neovim.
                            // panic!("heard ctrl");
                        }
                    },
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
                    event::KeyCode::Esc => self.mode = Mode::Normal,
                    event::KeyCode::Backspace => editor.backspace(),
                    event::KeyCode::Char(c) => editor.add_char(c),
                    event::KeyCode::Enter => {
                        Mode::process_cmd(self);
                        self.mode = Mode::Normal;
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
