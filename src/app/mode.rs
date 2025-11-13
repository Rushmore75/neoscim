use std::{cmp::min, fmt::Display};

use ratatui::{
    prelude,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

use crate::app::{
    app::App,
    error_msg::ErrorMessage,
    logic::calc::{CellType, LEN},
};

pub enum Mode {
    Insert(Chord),
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
            let cmd = &editor.as_string()[1..];
            let args = cmd.split_ascii_whitespace().collect::<Vec<&str>>();
            // we are guaranteed at least 1 arg
            if args.is_empty() {
                return;
            }

            match args[0] {
                "w" => {
                    // first try the passed argument as file
                    if let Some(arg) = args.get(1) {
                        if let Err(e) = app.grid.save_to(arg) {
                            app.error_msg = ErrorMessage::new(format!("{e}"));
                        } else {
                            // file saving was a success, adopt the provided file
                            // if we don't already have one (this is how vim works)
                            if let None = app.file {
                                app.file = Some(arg.into())
                            }
                        }
                    // then try the file that we opened the program with
                    } else if let Some(file) = &app.file {
                        if let Err(e) = app.grid.save_to(file) {
                            app.error_msg = ErrorMessage::new(format!("{e}"));
                        }
                    // you need to provide a file from *somewhere*
                    } else {
                        app.error_msg = ErrorMessage::new("No file selected");
                    }
                }
                // quit
                "q" => {
                    if app.grid.needs_to_be_saved() {
                        app.exit = false;
                        app.error_msg = ErrorMessage::new("File not saved");
                    } else {
                        app.exit = true
                    }
                }
                // force quit
                "q!" => {
                    app.exit = true;
                }
                "set" => {
                    if let Some(arg) = args.get(1) {
                        let parts: Vec<&str> = arg.split('=').collect();
                        if parts.len() != 2 {
                            app.error_msg = ErrorMessage::new("set <key>=<value>");
                            return;
                        }
                        let key = parts[0];
                        let value = parts[1];

                        app.vars.insert(key.to_owned(), value.to_owned());
                    }
                    app.error_msg = ErrorMessage::new("set <key>=<value>")
                }
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
                        app.grid.selected_cell.1 = min(app.grid.selected_cell.1.saturating_add(1), LEN - 1);
                        return;
                    }
                    // ^
                    'k' => {
                        app.grid.selected_cell.1 = app.grid.selected_cell.1.saturating_sub(1);
                        return;
                    }
                    // >
                    'l' => {
                        app.grid.selected_cell.0 = min(app.grid.selected_cell.0.saturating_add(1), LEN - 1);
                        return;
                    }
                    '0' => {
                        app.grid.selected_cell.0 = 0;
                        return;
                    }
                    // edit cell
                    'i' | 'a' => {
                        let (x, y) = app.grid.selected_cell;

                        let val = app.grid.get_cell_raw(x, y).as_ref().map(|f| f.to_string()).unwrap_or(String::new());

                        app.mode = Mode::Insert(Chord::from(val));
                    }
                    // replace cell
                    'r' => {
                        app.mode = Mode::Insert(Chord::from(String::new()));
                    }
                    'I' => { /* insert col before */ }
                    'A' => { /* insert col after */ }
                    'o' => { /* insert row below */ }
                    'O' => { /* insert row above */ }
                    'v' => app.mode = Mode::Visual(app.grid.selected_cell),
                    ':' => app.mode = Mode::Command(Chord::new(':')),
                    // loose chars will put you into chord mode
                    c => {
                        if let Mode::Normal = app.mode {
                            app.mode = Mode::Chord(Chord::new(c))
                        }
                    }
                }
                if let Mode::Visual((x1, y1)) = app.mode {
                    // TODO visual copy, paste, etc
                    let (x2, y2) = app.grid.selected_cell;

                    let (low_x, hi_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                    let (low_y, hi_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };

                    if key == 'd' {
                        for x in low_x..=hi_x {
                            for y in low_y..=hi_y {
                                app.grid.set_cell_raw::<CellType>((x, y), None);
                            }
                        }
                        app.mode = Mode::Normal
                    }
                }
            }
            Mode::Chord(chord) => {
                chord.add_char(key);

                // the chord starts with a :, send it over to be a command
                if chord.buf[0] == ':' {
                    app.mode = Mode::Command(Chord::new(':'));
                    return;
                }

                // Try and parse out a preceding number
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
                    Err(_) => {
                        let c = chord.as_string();
                        // match everything up to, and then the new key
                        match (&c[..c.len()-1], key) {
                            // delete cell under cursor
                            ("d", ' ') | ("d", 'w') => {
                                let loc = app.grid.selected_cell;
                                app.grid.set_cell_raw::<CellType>(loc, None);
                                app.mode = Mode::Normal;
                            }
                            // go to top of row
                            ("g", 'g') => {
                                app.grid.selected_cell.1 = 0;
                                app.mode = Mode::Normal;
                            }
                            // center screen to cursor
                            ("z", 'z') => {
                                app.screen.center_x(app.grid.selected_cell, &app.vars);
                                app.screen.center_y(app.grid.selected_cell, &app.vars);
                                app.mode = Mode::Normal;
                            }
                            // mark cell
                            ("m", i) => {
                                app.marks.insert(i, app.grid.selected_cell);
                                app.mode = Mode::Normal;
                            }
                            // goto marked cell
                            ("'", i) => {
                                if let Some(coords) = app.marks.get(&i) {
                                    app.grid.selected_cell = *coords;
                                }
                                app.mode = Mode::Normal;
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => todo!(),
        }
    }
}

pub struct Chord {
    buf: Vec<char>,
}

impl From<String> for Chord {
    fn from(value: String) -> Self {
        let b = value.as_bytes().iter().map(|f| *f as char).collect();
        Chord {
            buf: b,
        }
    }
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
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

impl Widget for &Chord {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        Paragraph::new(self.buf.iter().collect::<String>()).render(area, buf);
    }
}

#[test]
fn keybinds() {
    let mut app = App::new();

    assert_eq!(app.grid.selected_cell, (0,0));

    // start at B1
    app.grid.selected_cell = (1,1);
    assert_eq!(app.grid.selected_cell, (1,1));

    // gg
    app.mode = Mode::Chord(Chord::new('g'));
    Mode::process_key(&mut app, 'g');
    assert_eq!(app.grid.selected_cell, (1,0));

    // 0
    app.mode = Mode::Normal;
    Mode::process_key(&mut app, '0');
    assert_eq!(app.grid.selected_cell, (0,0));

    // 10l
    // this should mean all the directions work
    app.grid.selected_cell = (0,0);
    app.mode = Mode::Chord(Chord::new('1'));
    Mode::process_key(&mut app, '0');
    Mode::process_key(&mut app, 'l');
    assert_eq!(app.grid.selected_cell, (10,0));
}
