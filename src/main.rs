// #![feature(impl_trait_in_bindings)]

mod calc;
mod ctx;

use std::io;

use ratatui::{
    crossterm::event,
    layout::{Constraint, Layout},
    text::*,
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

struct App {
    exit: bool,
    grid: Grid,
    /// Buffer for key-chords
    chord_buf: String,
    editor: Option<Editor>,
}

impl Widget for &App {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        Paragraph::new("Status").render(area, buf);
    }
}

impl App {
    fn new() -> Self {
        Self {
            exit: false,
            grid: Grid::new(),
            chord_buf: String::new(),
            editor: None,
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
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(frame.area());

        if let Some(editor) = &self.editor {
            frame.render_widget(editor, layout[0]);
        } else {
            frame.render_widget(Paragraph::new("sc_rs"), layout[0]);
        }

        frame.render_widget(&self.grid, layout[1]);
        frame.render_widget(self, layout[2]);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            event::Event::Key(key_event) => match key_event.code {
                event::KeyCode::Enter => {
                    if let Some(editor) = &self.editor {
                        let loc= self.grid.selected_cell;

                        let val = editor.buf.trim().to_string();

                        // insert as number if at all possible
                        if let Ok(val) = val.parse::<f64>() {
                            self.grid.set_cell_raw(loc, val);
                        } else {
                            self.grid.set_cell_raw(loc, val);
                        };

                        self.editor = None;
                    }
                }
                event::KeyCode::Backspace => {
                    if let Some(editor) = &mut self.editor {
                        editor.buf.pop();
                    }
                }
                event::KeyCode::F(_) => todo!(),
                event::KeyCode::Char(c) => {

                    if let Some(editor) = &mut self.editor {
                        editor.buf += &c.to_string();
                        return Ok(());
                    }

                    if !self.chord_buf.is_empty() {}

                    match c {
                        'q' => self.exit = true,
                        // <
                        'h' => self.grid.selected_cell.0 = self.grid.selected_cell.0.saturating_sub(1),
                        // v
                        'j' => self.grid.selected_cell.1 = self.grid.selected_cell.1.saturating_add(1),
                        // ^
                        'k' => self.grid.selected_cell.1 = self.grid.selected_cell.1.saturating_sub(1),
                        // >
                        'l' => self.grid.selected_cell.0 = self.grid.selected_cell.0.saturating_add(1),
                        // edit cell
                        'i' | 'a' => {
                            let (x,y) = self.grid.selected_cell;
                            let starting_val = if let Some(val) = self.grid.get_cell_raw(x, y) {
                                val.as_raw_string()
                            } else {
                                String::new()
                            };
                            self.editor = Some(Editor::from(starting_val))
                        },
                        'I' => {/* insert col before */}
                        'A' => {/* insert col after */}
                        'o' => {/* insert row below */}
                        'O' => {/* insert row above */}
                        ':' => {/* enter command mode */}
                        c => {
                            // start entering c for words
                            self.chord_buf += &c.to_string();
                        }
                    }
                },
                _ => {}
            },
            _ => {}
            event::Event::Paste(_) => todo!(),
            event::Event::Resize(_, _) => todo!(),
        }
        Ok(())
    }
}

struct Editor {
    buf: String,
    cursor: usize,
}

impl From<String> for Editor {
    fn from(value: String) -> Self {
        Self {
            buf: value.to_string(),
            cursor: value.len(),
        }
    }
}

impl Widget for &Editor {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        Paragraph::new(self.buf.clone()).render(area, buf);
    }
}