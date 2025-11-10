mod calc;
mod ctx;

use std::{fmt::Display, io, path::PathBuf};

use ratatui::{
    crossterm::event,
    layout::{Constraint, Layout},
    widgets::{Paragraph, Widget},
    *,
};

use crate::calc::Grid;

#[test]
fn test_math() {
    use evalexpr::*;

    let mut grid = Grid::new();
    grid.set_cell("A0", 2.);
    grid.set_cell("B0", 1.);
    grid.set_cell("C0", "=A0+B0".to_string());

    assert_eq!(eval("1+2").unwrap(), Value::Int(3));

    let cell_text = &grid.get_cell("C0");
    if let Some(text) = cell_text {
        if text.is_equation() {
            println!("{}", text.as_raw_string());
            let display = grid.evaluate(&text.as_raw_string());
            assert_eq!(display, Some(3.));
            return;
        }
    }
    panic!("Should've found the value and returned");
}

fn main() -> Result<(), std::io::Error> {
    let term = ratatui::init();
    let mut app = App::new();
    app.grid.set_cell("A0", 10.);
    app.grid.set_cell("B1", 10.);
    app.grid.set_cell("C2", "=A0+B1".to_string());

    let res = app.run(term);
    ratatui::restore();
    return res;
}

enum Mode {
    Insert(Editor),
    Chord(Chord),
    Normal,
    Command(Chord),
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Insert(_) => write!(f, "- INSERT -"),
            Mode::Chord(_) => write!(f, "- CHORD -"),
            Mode::Normal => write!(f, "- NORMAL -"),
            Mode::Command(_) => write!(f, "- COMMAND -"),
        }
    }
}

impl Mode {
    fn process_key(app: &mut App, key: char) {
        match key {
            // <
            'h' => app.grid.selected_cell.0 = app.grid.selected_cell.0.saturating_sub(1),
            // v
            'j' => app.grid.selected_cell.1 = app.grid.selected_cell.1.saturating_add(1),
            // ^
            'k' => app.grid.selected_cell.1 = app.grid.selected_cell.1.saturating_sub(1),
            // >
            'l' => app.grid.selected_cell.0 = app.grid.selected_cell.0.saturating_add(1),
            // edit cell
            'i' | 'a' => {
                let (x, y) = app.grid.selected_cell;

                let val = app.grid.get_cell_raw(x, y).as_ref().map(|f| f.as_raw_string()).unwrap_or(String::new());

                app.mode = Mode::Insert(Editor::new(val, (x, y)));
            }
            'I' => { /* insert col before */ }
            'A' => { /* insert col after */ }
            'o' => { /* insert row below */ }
            'O' => { /* insert row above */ }
            ':' => app.mode = Mode::Command(Chord::new(':')),
            // loose chars will put you into chord mode
            c => app.mode = Mode::Chord(Chord::new(c)),
        }
    }
}

struct App {
    exit: bool,
    grid: Grid,
    mode: Mode,
    file: Option<PathBuf>,
}

impl Widget for &App {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        Paragraph::new(self.mode.to_string()).render(area, buf);
    }
}

impl App {
    fn new() -> Self {
        Self {
            exit: false,
            grid: Grid::new(),
            mode: Mode::Normal,
            file: None,
        }
    }

    fn run(&mut self, mut term: DefaultTerminal) -> Result<(), std::io::Error> {
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
        }

        frame.render_widget(&self.grid, layout[1]);
        frame.render_widget(self, bottom_split[1]);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match &mut self.mode {
            Mode::Chord(chord) => match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => self.mode = Mode::Normal,
                    event::KeyCode::Char(c) => {
                        chord.buf.push(c);
                        match chord.as_string()[0..chord.buf.len() - 1].parse::<usize>() {
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
                            _ => {}
                        }
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
            Mode::Command(editor) => match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Esc => {
                        // just cancel the operation
                        self.mode = Mode::Normal;
                    }
                    event::KeyCode::Enter => {
                        // [':', 'q']
                        match editor.buf[1] {
                            'w' => {}
                            'q' => self.exit = true,
                            _ => {}
                        }
                        self.mode = Mode::Normal;
                    }
                    event::KeyCode::Backspace => {
                        editor.buf.pop();
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

struct Editor {
    buf: String,
    cursor: usize,
    location: (usize, usize),
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

struct Chord {
    buf: Vec<char>,
}

impl Chord {
    fn new(inital: char) -> Self {
        let mut buf = Vec::new();
        buf.push(inital);

        Self {
            buf,
        }
    }

    fn backspace(&mut self) {
        self.buf.pop();
    }

    fn add_char(&mut self, c: char) {
        self.buf.push(c)
    }

    fn as_string(&self) -> String {
        self.buf.iter().collect()
    }
}

impl Widget for &Chord {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        Paragraph::new(self.buf.iter().collect::<String>()).render(area, buf);
    }
}
