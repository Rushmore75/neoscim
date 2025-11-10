use std::{io, path::PathBuf};

use ratatui::{DefaultTerminal, Frame, crossterm::event, layout::{self, Constraint, Layout, Rect}, prelude, style::{Color, Modifier, Style}, widgets::{Paragraph, Widget}};

use crate::app::{calc::{Grid, LEN}, mode::Mode};

pub struct App {
    exit: bool,
    pub grid: Grid,
    pub mode: Mode,
    file: Option<PathBuf>,
}

impl Widget for &App {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        let len = LEN as u16;

        let cell_height = 1;
        let cell_length = 5;


        let x_max = if area.width / cell_length > len {
            len - 1
        } else {
            area.width / cell_length
        };
        let y_max = if area.height / cell_height > len {
            len - 1
        } else {
            area.height / cell_height
        };

        for x in 0..x_max {
            for y in 0..y_max {
                let mut display = String::new();
                let mut style = Style::new().fg(Color::White);

                const ORANGE1: Color = Color::Rgb(200, 160, 0);
                const ORANGE2: Color = Color::Rgb(180, 130, 0);

                match (x == 0, y == 0) {
                    (true, true) => {
                        let (x,y) = self.grid.selected_cell;
                        let c = Grid::num_to_char(x);
                        display = format!("{y}{c}", );
                        style = Style::new().fg(Color::Green).bg(Color::Black);
                    },
                    (true, false) => {
                        // row names
                        display = y.to_string();
                        
                        let bg = if y%2==0 {
                            ORANGE1
                        } else {
                            ORANGE2
                        };
                        style = Style::new().fg(Color::White).bg(bg);

                    },
                    (false, true) => {
                        // column names
                        display = Grid::num_to_char(x as usize -1);

                        let bg = if x%2==0 {
                            ORANGE1
                        } else {
                            ORANGE2
                        };
 
                        style = Style::new().fg(Color::White).bg(bg)
                    },
                    (false, false) => {
                        // minus 1 because of header cells
                        let x_idx = x as usize -1;
                        let y_idx = y as usize -1;

                        if let Some(cell) = self.grid.get_cell_raw(x_idx, y_idx) {
                            display = cell.as_raw_string();

                            if cell.can_be_number() {
                                if let Some(val) = self.grid.evaluate(&cell.as_raw_string()) {
                                    display = val.to_string();
                                    style = Style::new().underline_color(Color::DarkGray).add_modifier(Modifier::UNDERLINED);
                                } else {
                                    // broken formulas
                                    if cell.is_equation() {
                                        style = Style::new().underline_color(Color::Red).add_modifier(Modifier::UNDERLINED)
                                    }
                                }
                            }
                        }
                        if (x_idx, y_idx) == self.grid.selected_cell {
                            style = Style::new()
                                .fg(Color::Black)
                                .bg(Color::White);
                            // modify the style of the cell you are editing
                            if let Mode::Insert(_) = self.mode {
                                style = style.add_modifier(Modifier::ITALIC).add_modifier(Modifier::BOLD);
                            }
                        }
                    }
                }

                let area = Rect::new(
                    area.x + (x * cell_length),
                    area.y + (y * cell_height),
                    cell_length,
                    cell_height,
                );

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
            .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
            .split(frame.area());

        let bottom_split = Layout::default()
            .direction(layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[2]);

        match &self.mode {
            Mode::Insert(editor) => {
                frame.render_widget(editor, layout[0]);
            }
            Mode::Command(editor) => {
                frame.render_widget(editor, bottom_split[0]);
            }
            Mode::Chord(chord) => frame.render_widget(chord, bottom_split[0]),
            Mode::Normal => frame.render_widget(
                Paragraph::new({
                    let (x, y) = self.grid.selected_cell;
                    let cell = self.grid.get_cell_raw(x, y).as_ref().map(|f| f.as_raw_string()).unwrap_or_default();
                    cell
                }),
                layout[0],
            ),
            Mode::Visual(start_pos) => {}
        }

        frame.render_widget(self, layout[1]);
        frame.render_widget(&self.mode, bottom_split[1]);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match &mut self.mode {
            Mode::Chord(chord) => match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => self.mode = Mode::Normal,
                    event::KeyCode::Char(c) => {
                        chord.add_char(c);
                        match chord.as_string()[0..chord.as_string().len() - 1].parse::<usize>() {
                            Ok(num) => match c {
                                'G' => {
                                    let sel = self.grid.selected_cell;
                                    self.grid.selected_cell = (sel.0, num);
                                    self.mode = Mode::Normal;
                                }
                                _ => {
                                    if c.is_alphabetic() {
                                        self.mode = Mode::Normal;
                                        for _ in 0..num {
                                            Mode::process_key(self, c);
                                        }
                                    }
                                }
                            },
                            Err(_) => match chord.as_string().as_str() {
                                "d " | "dw" => {
                                    let loc = self.grid.selected_cell;
                                    self.grid.set_cell_raw(loc, String::new());
                                    self.mode = Mode::Normal;
                                }
                                _ => {}
                            },
                        }
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
            Mode::Visual(start_pos) => {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        event::KeyCode::Char(c) => {
                            Mode::process_key(self, c);
                            todo!();
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
                        // [':', 'q']
                        match editor.as_string().as_bytes()[1] as char {
                            'w' => {}
                            'q' => self.exit = true,
                            _ => {}
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
        }

        Ok(())
    }
}
