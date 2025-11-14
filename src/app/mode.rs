use std::{cmp::min, fmt::Display, path::PathBuf};

use ratatui::{
    prelude,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

use crate::app::{
    app::App,
    error_msg::StatusMessage,
    logic::calc::LEN,
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
                            app.msg = StatusMessage::error(format!("{e}"));
                        } else {
                            // file saving was a success, adopt the provided file
                            // if we don't already have one (this is how vim works)
                            let path: PathBuf = arg.into();
                            app.msg = StatusMessage::info(format!("Saved file {}", path.file_name().map(|f| f.to_str().unwrap_or("n/a")).unwrap_or("n/a")));

                            if let None = app.file {
                                app.file = Some(path)
                            }
                        }
                    // then try the file that we opened the program with
                    } else if let Some(file) = &app.file {
                        if let Err(e) = app.grid.save_to(file) {
                            app.msg = StatusMessage::error(format!("{e}"));
                        } else {
                            app.msg = StatusMessage::info(format!("Saved file {}", file.file_name().map(|f| f.to_str().unwrap_or("n/a")).unwrap_or("n/a")));
                        }
                    // you need to provide a file from *somewhere*
                    } else {
                        app.msg = StatusMessage::error("No file selected");
                    }
                }
                // quit
                "q" => {
                    if app.grid.needs_to_be_saved() {
                        app.exit = false;
                        app.msg = StatusMessage::error("File not saved");
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
                            app.msg = StatusMessage::error("set <key>=<value>");
                            return;
                        }
                        let key = parts[0];
                        let value = parts[1];

                        app.vars.insert(key.to_owned(), value.to_owned());
                    }
                    app.msg = StatusMessage::error("set <key>=<value>")
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
                        let (x, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(x.saturating_sub(1), y);
                        return;
                    }
                    // v
                    'j' => {
                        let (x, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(x, min(y.saturating_add(1), LEN - 1));
                        return;
                    }
                    // ^
                    'k' => {
                        let (x, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(x, y.saturating_sub(1));
                        return;
                    }
                    // >
                    'l' => {
                        let (x, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(min(x.saturating_add(1), LEN - 1), y);
                        return;
                    }
                    '0' => {
                        let (_, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(0, y);
                        return;
                    }
                    // edit cell
                    'i' | 'a' => {
                        let (x, y) = app.grid.cursor();

                        let val = app.grid.get_cell_raw(x, y).as_ref().map(|f| f.to_string()).unwrap_or(String::new());

                        app.mode = Mode::Insert(Chord::from(val));
                    }
                    // replace cell
                    'r' => {
                        app.mode = Mode::Insert(Chord::from(String::new()));
                    }
                    // insert column before
                    'I' => {
                        app.grid.insert_column_before(app.grid.cursor());
                    }
                    // insert column after
                    'A' => {
                        app.grid.insert_column_after(app.grid.cursor());
                    }
                    // insert row below
                    'o' => {
                        app.grid.insert_row_below(app.grid.cursor());
                    }
                    // insert row above
                    'O' => {
                        app.grid.insert_row_above(app.grid.cursor());
                    }
                    'v' => app.mode = Mode::Visual(app.grid.cursor()),
                    ':' => app.mode = Mode::Command(Chord::new(':')),
                    'p' => {
                        app.clipboard.paste(&mut app.grid, true);
                        app.grid.apply_momentum(app.clipboard.momentum());
                        return;
                    }
                    // loose chars will put you into chord mode
                    c => {
                        if let Mode::Normal = app.mode {
                            app.mode = Mode::Chord(Chord::new(c))
                        }
                    }
                }
                if let Mode::Visual((x1, y1)) = app.mode {
                    // TODO visual copy, paste, etc
                    let (x2, y2) = app.grid.cursor();

                    match key {
                        'd' | 'x' => {
                            app.clipboard.clipboard_cut((x1,y1), (x2,y2), &mut app.grid);
                            app.mode = Mode::Normal
                        }
                        'y' => {
                            app.clipboard.clipboard_copy((x1, y1), (x2, y2), &app.grid);
                            app.msg = StatusMessage::info(format!("Yanked {} cells", app.clipboard.qty()));
                            app.mode = Mode::Normal
                        }
                        _ => {}
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
                            let (x, _) = app.grid.cursor();
                            app.grid.mv_cursor_to(x, num);
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
                        match (&c[..c.len() - 1], key) {
                            // delete cell under cursor
                            ("d", ' ') | ("d", 'w') => {
                                let loc = app.grid.cursor();
                                app.clipboard.clipboard_cut(loc, loc, &mut app.grid);
                                app.mode = Mode::Normal;
                            }
                            // go to top of row
                            ("g", 'g') => {
                                let (x, _) = app.grid.cursor();
                                app.grid.mv_cursor_to(x, 0);
                                app.mode = Mode::Normal;
                            }
                            // center screen to cursor
                            ("z", 'z') => {
                                app.screen.center_x(app.grid.cursor(), &app.vars);
                                app.screen.center_y(app.grid.cursor(), &app.vars);
                                app.mode = Mode::Normal;
                            }
                            // mark cell
                            ("m", i) => {
                                app.marks.insert(i, app.grid.cursor());
                                app.mode = Mode::Normal;
                            }
                            // goto marked cell
                            ("'", i) => {
                                if let Some((cx, cy)) = app.marks.get(&i) {
                                    app.grid.mv_cursor_to(*cx, *cy);
                                }
                                app.mode = Mode::Normal;
                            }
                            // copy 1 cell
                            ("y", 'y') => {
                                let point = app.grid.cursor();
                                app.clipboard.clipboard_copy(point, point, &app.grid);
                                app.mode = Mode::Normal;
                                app.msg = StatusMessage::info("Yanked 1 cell");
                            }
                            ("g", 'p') => {
                                app.clipboard.paste(&mut app.grid, false);
                                app.grid.apply_momentum(app.clipboard.momentum());
                                app.mode = Mode::Normal;
                                let plural = if app.clipboard.qty() > 1 {"cells"} else {"cell"};
                                app.msg = StatusMessage::info(format!("Pasted {plural}, no formatting"));
                        return;
                            }
                            _ => {}
                        }
                    }
                }
            }
            // IDK why it works but it does. Keystrokes are process somewhere else?
            Mode::Insert(_chord) => {},
            Mode::Command(_chord) => {},
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
fn movement_keybinds() {
    let mut app = App::new();

    assert_eq!(app.grid.cursor(), (0, 0));
    Mode::process_key(&mut app, 'j');
    assert_eq!(app.grid.cursor(), (0, 1));

    Mode::process_key(&mut app, 'l');
    assert_eq!(app.grid.cursor(), (1, 1));

    Mode::process_key(&mut app, 'k');
    assert_eq!(app.grid.cursor(), (1, 0));

    Mode::process_key(&mut app, 'h');
    assert_eq!(app.grid.cursor(), (0, 0));
}

#[test]
fn keybinds() {
    let mut app = App::new();

    // start at B1
    app.grid.mv_cursor_to(1, 1);
    assert_eq!(app.grid.cursor(), (1, 1));

    // gg
    app.mode = Mode::Chord(Chord::new('g'));
    Mode::process_key(&mut app, 'g');
    assert_eq!(app.grid.cursor(), (1, 0));

    // 0
    app.mode = Mode::Normal;
    Mode::process_key(&mut app, '0');
    assert_eq!(app.grid.cursor(), (0, 0));

    // 10l
    // this should mean all the directions work
    app.grid.mv_cursor_to(0, 0);
    app.mode = Mode::Chord(Chord::new('1'));
    Mode::process_key(&mut app, '0');
    Mode::process_key(&mut app, 'l');
    assert_eq!(app.grid.cursor(), (10, 0));
}
