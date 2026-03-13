use std::{cmp::min, fmt::Display, fs, path::PathBuf, process::Command};

use ratatui::{
    layout::{self, Constraint, Layout, Margin, Offset, Rect},
    prelude,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{
    app::App,
    error_msg::StatusMessage,
    logic::{
        cell::{Cell, FormatRule},
        grid::{GRID_LEN, Grid},
    },
};

pub const ALL_COLORS: [Color; 16] = [
    Color::Black,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Gray,
    Color::DarkGray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::White,
];

pub enum Mode {
    Insert(EditBuffer),
    Chord(EditBuffer),
    Normal,
    Command(EditBuffer),
    Visual((usize, usize)),
    VisualCmd((usize, usize), EditBuffer),
    Formatting(FormatEditor),
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "NORMAL"),
            Mode::Insert(_) => write!(f, "INSERT"),
            Mode::Chord(_) => write!(f, "CHORD"),
            Mode::Command(_) => write!(f, "COMMAND"),
            Mode::Visual(_) => write!(f, "VISUAL"),
            Mode::VisualCmd(_, _) => write!(f, "V-CMD"),
            Mode::Formatting(_) => write!(f, "FMT"),
        }
    }
}

impl Mode {
    pub fn get_style(&self) -> Style {
        match self {
            // Where you are typing
            Mode::Insert(_) => Style::new().fg(Color::White).bg(Color::Blue),
            Mode::Command(_) => Style::new().fg(Color::Black).bg(Color::Magenta),
            Mode::VisualCmd(_, _) => Style::new().fg(Color::Black).bg(Color::Yellow),
            Mode::Chord(_) => Style::new().fg(Color::Black).bg(Color::LightBlue),
            Mode::Formatting(_) => Style::new().fg(Color::White).bg(Color::Cyan),
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
                "format" => app.grid.set_mode_formatting(),
                "edit" => app.grid.set_mode_editing(),
                "w" => {
                    // first try the passed argument as file
                    if let Some(arg) = args.get(1) {
                        let path: PathBuf = arg.into();

                        // TODO Check if the file we are writing to exists, since
                        // this code path already knows that we are writing to a new file.
                        // We might be accidentally overwriting something else.

                        if let Err(e) = app.grid.save_to(&path) {
                            app.msg = StatusMessage::error(format!("{e}"));
                        } else {
                            // file saving was a success, adopt the provided file
                            // if we don't already have one (this is how vim works)
                            app.msg = StatusMessage::info(format!(
                                "Saved file {}",
                                path.file_name().map(|f| f.to_str().unwrap_or("n/a")).unwrap_or("n/a")
                            ));

                            if app.file.is_none() {
                                app.file = Some(path)
                            }
                        }
                    // then try the file that we opened the program with
                    } else if let Some(file) = &app.file {
                        if let Err(e) = app.grid.save_to(file) {
                            app.msg = StatusMessage::error(format!("{e}"));
                        } else {
                            app.msg = StatusMessage::info(format!(
                                "Saved file {}",
                                file.file_name().map(|f| f.to_str().unwrap_or("n/a")).unwrap_or("n/a")
                            ));
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
        if let Mode::VisualCmd(pos, editor) = &mut app.mode {
            let cmd = &editor.as_string()[1..];
            let args = cmd.split_ascii_whitespace().collect::<Vec<&str>>();
            if args.is_empty() {
                return;
            }

            // These values are going to be used in probably all
            // the commands related to ranges, we will just write
            // logic here first, once.
            let (x1, y1) = pos;
            let (x1, y1) = (*x1, *y1);
            let (x2, y2) = app.grid.cursor();
            let (low_x, hi_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
            let (low_y, hi_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };

            let mut save_range = |to: &str| {
                let mut g = Grid::new();
                g.transact_on_grid(|grid| {
                    for (i, x) in (low_x..=hi_x).enumerate() {
                        for (j, y) in (low_y..=hi_y).enumerate() {
                            grid.set_cell_raw((i, j), app.grid.get_cell_raw(x, y).clone());
                        }
                    }
                });
                if let Err(_e) = g.save_to(to) {
                    app.msg = StatusMessage::error("Failed to save file");
                }
            };

            let get_project_name = || {
                if let Some(file) = &app.file {
                    if let Some(name) = file.file_name() {
                        if let Some(name) = name.to_str() {
                            return name;
                        }
                    }
                }
                return "unknown";
            };

            match args[0] {
                "f" | "fill" => {
                    app.grid.transact_on_grid(|grid| {
                        for (i, x) in (low_x..=hi_x).enumerate() {
                            for (j, y) in (low_y..=hi_y).enumerate() {
                                let arg = args
                                    .get(1)
                                    .map(|s| s.replace("xi", &i.to_string()))
                                    .map(|s| s.replace("yi", &j.to_string()))
                                    .map(|s| s.replace("x", &x.to_string()))
                                    .map(|s| s.replace("y", &y.to_string()));
                                grid.merge_in_value((x, y), arg);
                            }
                        }
                    });

                    app.mode = Mode::Normal
                }
                "export" => {
                    if let Some(arg1) = args.get(1) {
                        save_range(arg1);
                    } else {
                        app.msg = StatusMessage::error("export <path.csv>")
                    }
                    app.mode = Mode::Normal
                }
                "plot" => {
                    // Use gnuplot to plot the selected data.
                    // * Temp data will be stored in /tmp/
                    // * Output will either be plot.png or a name that you pass in
                    let output_filename = if let Some(arg1) = args.get(1) { arg1 } else { "plot.png" };

                    save_range("/tmp/plot.csv");
                    let plot = include_str!("../../template.gnuplot");
                    let s = plot.replace("$FILE", "/tmp/plot.csv");
                    let s = s.replace("$TITLE", get_project_name());
                    let s = s.replace("$XLABEL", "hard-coded x");
                    let s = s.replace("$YLABEL", "hard-coded y");
                    let s = s.replace("$OUTPUT", "/tmp/output.png");
                    let _ = fs::write("/tmp/plot.p", s);

                    let cmd_res = Command::new("gnuplot").arg("/tmp/plot.p").output();
                    if let Err(err) = cmd_res {
                        match err.kind() {
                            std::io::ErrorKind::NotFound => {
                                app.msg = StatusMessage::error("Error - Is gnuplot installed?")
                            }
                            _ => app.msg = StatusMessage::error(format!("{err}")),
                        };
                    } else {
                        let _ = fs::copy("/tmp/output.png", output_filename);
                        app.msg = StatusMessage::info(format!("Created {output_filename}. Artifacts are in /tmp"));
                    }
                    app.mode = Mode::Normal
                }
                _ => {}
            }
        }
    }

    // Why is this function not just inside app.rs's handle_events()?
    pub fn process_key(app: &mut App, key: char) {
        match &mut app.mode {
            Mode::Normal | Mode::Visual(_) => {
                match key {
                    // <
                    'h' | 'b' => {
                        let (x, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(x.saturating_sub(1), y);
                        return;
                    }
                    // v
                    'j' => {
                        let (x, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(x, min(y.saturating_add(1), GRID_LEN - 1));
                        return;
                    }
                    // ^
                    'k' => {
                        let (x, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(x, y.saturating_sub(1));
                        return;
                    }
                    // >
                    'l' | 'w' => {
                        let (x, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(min(x.saturating_add(1), GRID_LEN - 1), y);
                        return;
                    }
                    '0' => {
                        let (_, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(0, y);
                        return;
                    }
                    // Go to end of row
                    '$' => {
                        let (_, y) = app.grid.cursor();
                        app.grid.mv_cursor_to(GRID_LEN - 1, y);
                        return;
                    }
                    // Go to bottom of column
                    'G' => {
                        let (x, _) = app.grid.cursor();
                        app.grid.mv_cursor_to(x, GRID_LEN - 1);
                        return;
                    }
                    // edit cell
                    'i' | 'a' => {
                        let (x, y) = app.grid.cursor();

                        let val = app.grid.get_cell_display(x, y);

                        match app.grid.get_mode() {
                            super::logic::grid::GridType::Values => app.mode = Mode::Insert(EditBuffer::from(val)),
                            super::logic::grid::GridType::Formatting => {
                                let (x, y) = app.grid.cursor();
                                let cell = app.grid.get_cell_raw(x, y);
                                let cell = if let Some(cell) = cell { cell.to_owned() } else { Cell::default() };
                                app.mode = Mode::Formatting(FormatEditor::new(cell));
                            }
                        }
                    }
                    // replace cell
                    'r' => {
                        app.mode = Mode::Insert(EditBuffer::from(String::new()));
                    }
                    // insert column before
                    'I' => {
                        app.grid.insert_column_before(app.grid.cursor());
                    }
                    // insert column after
                    'A' => {
                        let c = app.grid.cursor();
                        app.grid.insert_column_after(c);
                        app.grid.mv_cursor_to(c.0 + 1, c.1);
                    }
                    // insert row below
                    'o' => {
                        let c = app.grid.cursor();
                        app.grid.insert_row_below(c);
                        app.grid.mv_cursor_to(c.0, c.1 + 1);
                    }
                    // insert row above
                    'O' => {
                        app.grid.insert_row_above(app.grid.cursor());
                    }
                    'v' => app.mode = Mode::Visual(app.grid.cursor()),
                    ':' => {
                        if let Self::Visual(pos) = app.mode {
                            app.mode = Mode::VisualCmd(pos, EditBuffer::new(':'));
                        } else {
                            app.mode = Mode::Command(EditBuffer::new(':'))
                        }
                    }
                    // undo
                    'u' => {
                        app.grid.undo();
                    }
                    // paste
                    'p' => {
                        app.clipboard.paste(&mut app.grid, true);
                        app.grid.apply_momentum(app.clipboard.momentum());
                        return;
                    }
                    // loose chars will put you into chord mode
                    c => {
                        if let Mode::Normal = app.mode {
                            app.mode = Mode::Chord(EditBuffer::new(c))
                        }
                    }
                }
                if let Mode::Visual((x1, y1)) = app.mode {
                    // TODO visual copy, paste, etc
                    let (x2, y2) = app.grid.cursor();

                    match key {
                        'd' | 'x' => {
                            app.clipboard.clipboard_cut((x1, y1), (x2, y2), &mut app.grid);
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
                    app.mode = Mode::Command(EditBuffer::new(':'));
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
                            // Go to the bottom of the current window
                            ("g", 'G') => {
                                let (x, _) = app.grid.cursor();
                                let (_, y_height) = app.screen.get_screen_size(&app.vars);
                                let y_origin = app.screen.scroll_y();

                                app.grid.mv_cursor_to(x, y_origin + y_height);
                                app.mode = Mode::Normal;
                                return;
                            }
                            // Go to the right edge of the current window
                            ("g", '$') => {
                                let (_, y) = app.grid.cursor();
                                let (x_width, _) = app.screen.get_screen_size(&app.vars);
                                let x_origin = app.screen.scroll_x();

                                app.grid.mv_cursor_to(x_origin + x_width, y);
                                app.mode = Mode::Normal;
                            }
                            // Go to the left edge of the current window
                            ("g", '0') => {
                                let (_, y) = app.grid.cursor();
                                let x_origin = app.screen.scroll_x();

                                app.grid.mv_cursor_to(x_origin, y);
                                app.mode = Mode::Normal;
                                return;
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
                                let plural = if app.clipboard.qty() > 1 { "cells" } else { "cell" };
                                app.msg = StatusMessage::info(format!("Pasted {plural}, no formatting"));
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            }
            // Keys are process in the handle_event method in App for these
            Mode::Formatting(_cell) => {}
            Mode::Insert(_chord) => {}
            Mode::Command(_chord) => {}
            Mode::VisualCmd(_pos, _chord) => {}
        }
    }

    pub fn render(&self, f: &mut ratatui::Frame, area: prelude::Rect, cell: String) {
        match &self {
            Mode::Insert(editor) => {
                f.render_widget(editor, area);
            }
            Mode::Command(editor) => {
                f.render_widget(editor, area);
            }
            Mode::Chord(chord) => f.render_widget(chord, area),
            Mode::Normal => f.render_widget(Paragraph::new(cell), area),
            Mode::Visual(_) => {}
            Mode::VisualCmd(_, editor) => f.render_widget(editor, area),
            Mode::Formatting(fmt) => {
                // this draws over other objects :)
                let a = f.area();
                let width = min(25, a.width); // don't draw oob
                let height = min(20, a.height);
                let xpos = (a.width / 2).saturating_sub(width / 2); // centered
                let ypos = (a.height / 2).saturating_sub(height / 2);
                let area = Rect::new(xpos, ypos, width, height);
                f.render_widget(fmt, area);
            }
        }
    }
}

pub struct EditBuffer {
    buf: Vec<char>,
}

impl From<String> for EditBuffer {
    fn from(value: String) -> Self {
        let b = value.as_bytes().iter().map(|f| *f as char).collect();
        EditBuffer { buf: b }
    }
}

impl EditBuffer {
    pub fn new(inital: char) -> Self {
        let buf = vec![inital];

        Self { buf }
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

impl Widget for &EditBuffer {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer) {
        Paragraph::new(self.buf.iter().collect::<String>()).render(area, buf);
    }
}

pub struct FormatEditor {
    pub mode: FormatEditorMode,
    pub cell: Cell,
}

impl FormatEditor {
    fn new(cell: Cell) -> Self {
        Self { mode: FormatEditorMode::Viewer(RulesViewer::default()), cell }
    }
}

pub enum FormatEditorMode {
    Viewer(RulesViewer),
    Editor(RuleEditor),
}

#[derive(Default)]
pub struct RulesViewer {
    pub index: u16,
}

pub enum EditingState {
    Selecting(usize),
    Value(EditBuffer),
    Sign(usize),
    FG(usize),
    BG(usize),
}

pub struct RuleEditor {
    pub editing: EditingState,
    pub rule: FormatRule<f64>,
    pub cell_rule_index: usize,
}

impl RuleEditor {
    pub fn new(rule: FormatRule<f64>, idx: usize) -> Self {
        Self { editing: EditingState::Selecting(0), rule, cell_rule_index: idx }
    }
}

impl Widget for &FormatEditor {
    fn render(self, area: prelude::Rect, buf: &mut prelude::Buffer)
    where
        Self: Sized,
    {
        let secondary = Style::new().fg(Color::DarkGray).bg(Color::Black);
        let primary = Style::new().fg(Color::White).bg(Color::Black);
        let primary_inverse = Style::default().fg(Color::Black).bg(Color::White);

        let title;
        let block = Block::default()
            .border_type(ratatui::widgets::BorderType::Rounded)
            .borders(Borders::all())
            .style(secondary);

        ratatui::widgets::Clear.render(area, buf);
        let inner = area.inner(Margin::new(1, 1));
        block.render(area, buf);

        match &self.mode {
            FormatEditorMode::Viewer(rules_viewer) => {
                title = "Format Rules";
                let line = Rect::new(inner.x, inner.y, inner.width, 1);
                let display = self.cell.value_string();
                let display = if display.is_empty() { "Empty".to_string() } else { display };
                Paragraph::new(display).style(secondary).render(line, buf);

                let mut l_arrow = line.offset(Offset { x: -1, y: rules_viewer.index as i32 + 1 });
                l_arrow.width = 1;
                let r_arrow = l_arrow.offset(Offset { x: area.width as i32 - 1, y: 0 });
                Paragraph::new("→").style(primary).render(l_arrow, buf);
                Paragraph::new("←").style(primary).render(r_arrow, buf);

                let scroll = 0; // For later when we might have longer rules lists

                self.cell.formatting.rules.iter().skip(scroll).zip(1..inner.height).for_each(|(fmt, offset)| {
                    let line = line.offset(Offset { x: 0, y: offset as i32 });
                    fmt.render(line, buf);
                });

                let line = line.offset(Offset { x: 0, y: inner.height.into() });
                Paragraph::new("'s' to save").centered().style(secondary).render(line, buf);
            }
            FormatEditorMode::Editor(rule_editor) => {
                match &rule_editor.editing {
                    EditingState::Selecting(index) => {
                        title = "Formatter";
                        let mut sign_color = primary;
                        let mut value_color = primary;
                        let mut fg_color = primary;
                        let mut bg_color = primary;
                        let mut yes_color = primary;
                        let mut no_color = primary;

                        let layout = Layout::default()
                            .direction(layout::Direction::Vertical)
                            .constraints([
                                // title
                                Constraint::Max(1),
                                // comparitor
                                Constraint::Max(1),
                                // then color:
                                Constraint::Max(1),
                                // colors
                                Constraint::Max(1),
                                Constraint::Max(1),
                                // Ok / Cancel
                                Constraint::Max(1),
                            ])
                            .split(inner);

                        let title = layout[0];
                        let comparitor = layout[1];
                        let then = layout[2];
                        let fg = layout[3];
                        let bg = layout[4];
                        let submit = layout[5];

                        match index {
                            0 => {
                                sign_color = primary_inverse;
                            }
                            1 => {
                                value_color = primary_inverse;
                            }
                            2 => {
                                fg_color = primary_inverse;
                            }
                            3 => {
                                bg_color = primary_inverse;
                            }
                            4 => yes_color = primary_inverse,
                            5 => no_color = primary_inverse,
                            _ => {}
                        }

                        Paragraph::new("If value is:").style(secondary).render(title, buf);
                        let comp = Layout::default()
                            .direction(layout::Direction::Horizontal)
                            .constraints([Constraint::Ratio(1, 2); 2])
                            .split(comparitor);
                        Paragraph::new(rule_editor.rule.sign_char().to_string())
                            .centered()
                            .style(sign_color)
                            .render(comp[0], buf);
                        Paragraph::new(rule_editor.rule.get_threashold().to_string())
                            .centered()
                            .style(value_color)
                            .render(comp[1], buf);
                        Paragraph::new("then color:").style(secondary).render(then, buf);
                        let fg = Layout::default()
                            .direction(layout::Direction::Horizontal)
                            .constraints([Constraint::Max(3), Constraint::Fill(1)])
                            .split(fg);
                        Paragraph::new("FG").style(secondary).render(fg[0], buf);
                        Paragraph::new(rule_editor.rule.style_string().0).style(fg_color).render(fg[1], buf);
                        let bg = Layout::default()
                            .direction(layout::Direction::Horizontal)
                            .constraints([Constraint::Max(3), Constraint::Fill(1)])
                            .split(bg);
                        Paragraph::new("BG").style(secondary).render(bg[0], buf);
                        Paragraph::new(rule_editor.rule.style_string().1).style(bg_color).render(bg[1], buf);
                        let sub = Layout::default()
                            .direction(layout::Direction::Horizontal)
                            .constraints([Constraint::Fill(1), Constraint::Fill(1), Constraint::Fill(1)])
                            .split(submit);
                        Paragraph::new("Submit?").style(secondary).render(sub[0], buf);
                        Paragraph::new("Yes").style(yes_color).render(sub[1], buf);
                        Paragraph::new("No").style(no_color).render(sub[2], buf);
                    }
                    EditingState::Value(edit) => {
                        title = "Value";
                        if edit.as_string().is_empty() {
                            Paragraph::new("xxx")
                        } else {
                            Paragraph::new(edit.as_string())
                        }
                        .centered()
                        .style(primary)
                        .render(inner, buf);
                    }
                    EditingState::Sign(index) => {
                        title = "Choose Sign";
                        let mut eq_color = primary;
                        let mut gt_color = primary;
                        let mut lt_color = primary;

                        let layout = Layout::default()
                            .direction(layout::Direction::Vertical)
                            .constraints([
                                // title
                                Constraint::Max(1),
                                // comparitor
                                Constraint::Max(1),
                                // then color:
                                Constraint::Max(1),
                                // colors
                                Constraint::Max(1),
                            ])
                            .split(inner);
                        let sign = layout[0];
                        let eq = layout[1];
                        let gt = layout[2];
                        let lt = layout[3];

                        match index {
                            0 => {
                                eq_color = primary_inverse;
                            }
                            1 => {
                                gt_color = primary_inverse;
                            }
                            2 => {
                                lt_color = primary_inverse;
                            }
                            _ => {}
                        }

                        Paragraph::new("Operator").style(secondary).render(sign, buf);
                        Paragraph::new("Equals (=)").style(eq_color).render(eq, buf);
                        Paragraph::new("Greater than (>)").style(gt_color).render(gt, buf);
                        Paragraph::new("Less than (<)").style(lt_color).render(lt, buf);
                    }
                    EditingState::BG(index) | EditingState::FG(index) => {
                        title = "Select Color";
                        let mut area = Rect::new(inner.x, inner.y, inner.width, 1);
                        for (i, c) in ALL_COLORS.iter().enumerate() {
                            let style = if i == *index { primary_inverse } else { primary };
                            Paragraph::new(c.to_string()).style(style).render(area, buf);
                            area = area.offset(Offset { x: 0, y: 1 });
                        }
                    }
                }
            }
        }

        let line = Rect::new(inner.x, inner.y - 1, title.len() as u16, 1);
        Paragraph::new(title).centered().style(primary).render(line, buf);
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
    app.mode = Mode::Chord(EditBuffer::new('g'));
    Mode::process_key(&mut app, 'g');
    assert_eq!(app.grid.cursor(), (1, 0));

    // 0
    app.mode = Mode::Normal;
    Mode::process_key(&mut app, '0');
    assert_eq!(app.grid.cursor(), (0, 0));

    // 10l
    // this should mean all the directions work
    app.grid.mv_cursor_to(0, 0);
    app.mode = Mode::Chord(EditBuffer::new('1'));
    Mode::process_key(&mut app, '0');
    Mode::process_key(&mut app, 'l');
    assert_eq!(app.grid.cursor(), (10, 0));
}
