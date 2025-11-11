use std::{cmp::{max, min}, fmt::Display};

use ratatui::{
    prelude, style::{Color, Style}, widgets::{Paragraph, Widget}
};

use crate::app::{app::App, calc::LEN, error_msg::ErrorMessage};

pub enum Mode {
    Insert(Editor),
    Chord(Chord),
    Normal,
    Command(Chord),
    Visual((usize, usize)),
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "NORMAL"),
            Mode::Insert(_) => write!(f, "INSERT"),
            Mode::Chord(_) => write!(f, "CHORD"),
            Mode::Command(_) => write!(f, "COMMAND"),
            Mode::Visual(_) => write!(f, "VISUAL"),
        }
    }
}

impl Mode {

    pub fn get_style(&self) -> Style {
        match self {
            // Where you are typing
            Mode::Insert(_) => Style::new().fg(Color::White).bg(Color::Blue),
            Mode::Command(_) => Style::new().fg(Color::Black).bg(Color::Magenta),
            Mode::Chord(_) => Style::new().fg(Color::Black).bg(Color::LightBlue),
            // Movement-based modes
            Mode::Visual(_) => Style::new().fg(Color::Yellow),
            Mode::Normal => Style::new().fg(Color::Green),
        }
    }


    pub fn process_cmd(app: &mut App) {
        if let Mode::Command(editor) = &mut app.mode {
            // [':', 'q']
            match editor.as_string().as_bytes()[1] as char {
                'w' => {
                    if let Some(file) = &app.file {
                        unimplemented!("Figure out how we want to save Grid to a csv or something")
                    } else {
                        app.error_msg = ErrorMessage::new("No file selected");
                    }
                }
                'q' => app.exit = true,
                _ => {}
            }
        }
    }

    pub fn process_key(app: &mut App, key: char) {
        match &mut app.mode {
            Mode::Normal | Mode::Visual(_) => {
                match key {
                    // <
                    'h' => {
                        app.grid.selected_cell.0 = app.grid.selected_cell.0.saturating_sub(1);
                        return;
                    }
                    // v
                    'j' => {
                        app.grid.selected_cell.1 = min(app.grid.selected_cell.1.saturating_add(1), LEN);
                        return;
                    }
                    // ^
                    'k' => {
                        app.grid.selected_cell.1 = app.grid.selected_cell.1.saturating_sub(1);
                        return;
                    }
                    // >
                    'l' => {
                        app.grid.selected_cell.0 = min(app.grid.selected_cell.0.saturating_add(1), LEN);
                        return;
                    }
                    '0' => {
                        app.grid.selected_cell.0 = 0;
                        return;
                    }
                    // edit cell
                    'i' | 'a' => {
                        let (x, y) = app.grid.selected_cell;

                        let val =
                            app.grid.get_cell_raw(x, y).as_ref().map(|f| f.to_string()).unwrap_or(String::new());

                        app.mode = Mode::Insert(Editor::new(val, (x, y)));
                    }
                    'I' => { /* insert col before */ }
                    'A' => { /* insert col after */ }
                    'o' => { /* insert row below */ }
                    'O' => { /* insert row above */ }
                    'v' => app.mode = Mode::Visual(app.grid.selected_cell),
                    ':' => app.mode = Mode::Command(Chord::new(':')),
                    // loose chars will put you into chord mode
                    c => app.mode = Mode::Chord(Chord::new(c)),
                }
                if let Mode::Visual(v) = app.mode {
                    // TODO visual delete, copy, paste, etc
                }
            }
            Mode::Chord(chord) => {
                chord.add_char(key);
                match chord.as_string()[0..chord.as_string().len() - 1].parse::<usize>() {
                    // For chords that can take a numeric input
                    Ok(num) => match key {
                        'G' => {
                            let sel = app.grid.selected_cell;
                            app.grid.selected_cell = (sel.0, num);
                            app.mode = Mode::Normal;
                        }
                        _ => {
                            if key.is_alphabetic() {
                                app.mode = Mode::Normal;
                                for _ in 0..num {
                                    Mode::process_key(app, key);
                                }
                            }
                        }
                    },
                    Err(_) => match chord.as_string().as_str() {
                        "d " | "dw" => {
                            let loc = app.grid.selected_cell;
                            app.grid.set_cell_raw(loc, String::new());
                            app.mode = Mode::Normal;
                        }
                        "gg" => {
                            app.grid.selected_cell.1 = 0;
                            app.mode = Mode::Normal;
                        }
                        _ => {}
                    },
                }
            }
            _ => todo!(),
        }
    }
}

pub struct Editor {
    pub buf: String,
    cursor: usize,
    pub location: (usize, usize),
}

impl Editor {
    fn new(value: String, loc: (usize, usize)) -> Self {
        Self {
            buf: value.to_string(),
            cursor: value.len(),
            location: loc,
        }
    }
}

impl Widget for &Editor {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        // TODO add visual cursor
        Paragraph::new(self.buf.clone()).render(area, buf);
    }
}

pub struct Chord {
    buf: Vec<char>,
}

impl Chord {
    pub fn new(inital: char) -> Self {
        let mut buf = Vec::new();
        buf.push(inital);

        Self {
            buf,
        }
    }

    pub fn backspace(&mut self) {
        self.buf.pop();
    }

    pub fn add_char(&mut self, c: char) {
        self.buf.push(c)
    }

    pub fn as_string(&self) -> String {
        self.buf.iter().collect()
    }
}

impl Widget for &Chord {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        Paragraph::new(self.buf.iter().collect::<String>()).render(area, buf);
    }
}
