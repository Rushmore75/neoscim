use std::{
    cmp::{max, min},
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

use evalexpr::*;

use crate::app::{
    logic::{
        calc::internal::CellGrid, cell::{CSV_DELIMITER, CellType}, ctx
    }
};

#[cfg(test)]
use crate::app::app::App;
#[cfg(test)]
use crate::app::mode::Mode;

pub fn get_header_size() -> usize {
    let row_header_width = LEN.to_string().len();
    row_header_width
}

pub const LEN: usize = 1001;
pub const CSV_EXT: &str = "csv";
pub const CUSTOM_EXT: &str = "nscim";

mod internal {
    use crate::app::logic::{calc::LEN, cell::CellType};

    #[derive(Clone)]
    pub struct CellGrid {
        //  a b c ...
        // 0
        // 1
        // 2
        // ...
        cells: Vec<Vec<Option<CellType>>>,
    }

    impl CellGrid {
        pub fn new() -> Self {
            let mut a = Vec::with_capacity(LEN);
            for _ in 0..LEN {
                let mut b = Vec::with_capacity(LEN);
                for _ in 0..LEN {
                    b.push(None)
                }
                a.push(b)
            }
            Self { cells: a }
        }

        pub fn insert_row(&mut self, y: usize) {
            for x in 0..LEN {
                self.cells[x].insert(y, None);
                self.cells[x].pop();
            }
        }

        pub fn insert_column(&mut self, x: usize) {
            let mut v = Vec::with_capacity(LEN);
            for _ in 0..LEN {
                v.push(None);
            }
            // let clone = self.grid_history[self.current_grid].clone();
            self.cells.insert(x, v);
            // keep the grid LEN
            self.cells.pop();
        }

        pub fn get_cell_raw(&self, x: usize, y: usize) -> &Option<CellType> {
            if x >= LEN || y >= LEN {
                return &None;
            }
            &self.cells[x][y]
        }
        pub fn set_cell_raw<T: Into<CellType>>(&mut self, (x, y): (usize, usize), val: Option<T>) {
            // TODO check oob
            self.cells[x][y] = val.map(|v| v.into());
        }
        /// Iterate over the entire grid and see where
        /// the farthest modified cell is.
        #[must_use]
        pub fn max(&self) -> (usize, usize) {
            let mut max_x = 0;
            let mut max_y = 0;

            for (xi, x) in self.cells.iter().enumerate() {
                for (yi, cell) in x.iter().enumerate() {
                    if cell.is_some() {
                        if yi > max_y {
                            max_y = yi
                        }
                        if xi > max_x {
                            max_x = xi
                        }
                    }
                }
            }

            (max_x, max_y)
        }

        #[must_use]
        pub fn max_y_at_x(&self, x: usize) -> usize {
            let mut max_y = 0;
            if let Some(col) = self.cells.get(x) {
                // we could do fancy things like .take_while(not null) but then
                // we would have to deal with empty cells and stuff, which sounds
                // boring. This will be fast "enough", considering the grid is
                // probably only like 1k cells
                for (yi, cell) in col.iter().enumerate() {
                    if cell.is_some() {
                        if yi > max_y {
                            max_y = yi
                        }
                    }
                }
            }
            max_y
        }
    }
}

pub struct Grid {
    /// Which grid in history are we currently on
    current_grid: usize,
    /// An array of grids, thru history
    grid_history: Vec<CellGrid>,
    /// (X, Y)
    selected_cell: (usize, usize),
    /// Have unsaved modifications been made?
    dirty: bool,
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid").field("cells", &"Too many to print").finish()
    }
}

impl Grid {
    pub fn new() -> Self {
        let x = CellGrid::new();

        Self { current_grid: 0, grid_history: vec![x], selected_cell: (0, 0), dirty: false }
    }

    pub fn new_from_file(file: &mut File) -> std::io::Result<Self> {
        let mut grid = Self::new();

        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        for (yi, line) in buf.lines().enumerate() {
            let cells = Self::parse_csv_line(line);

            for (xi, cell) in cells.into_iter().enumerate() {
                // This gets automatically duck-typed
                grid.set_cell_raw((xi, yi), cell);
            }
        }

        // force dirty back off, we just read the data so it's gtg
        grid.dirty = false;

        Ok(grid)
    }

    /// Save file to `path` as a csv. Path with have `csv` appended to it if it
    /// does not already have the extension.
    pub fn save_to(&mut self, path: impl Into<PathBuf>) -> std::io::Result<()> {
        let mut path = path.into();

        let resolve_values;

        match path.extension() {
            Some(ext) => match ext.to_str() {
                Some(CSV_EXT) => {
                    resolve_values = true;
                }
                Some(CUSTOM_EXT) => {
                    resolve_values = false;
                }
                _ => {
                    // File as an extension but isn't ours.
                    // Save as csv-like
                    resolve_values = true;
                }
            },
            None => {
                // File has no extension. Save it as our file type
                resolve_values = false;
                path.add_extension(CUSTOM_EXT);
            }
        }

        let mut f = fs::OpenOptions::new().write(true).append(false).truncate(true).create(true).open(path)?;
        let (mx, my) = self.get_grid().max();
        for y in 0..=my {
            for x in 0..=mx {
                let cell = &self.get_grid().get_cell_raw(x, y);

                // newline after the cell, because it's end of line.
                // else, just put a comma after the cell.
                let is_last = x == mx;
                let delim = if is_last { '\n' } else { CSV_DELIMITER };

                let data = if let Some(cell) = cell {
                    if let Ok(val) = self.evaluate(&cell.to_string())
                        && resolve_values
                    {
                        format!("{}{}", val.to_string(), delim)
                    } else {
                        format!("{}{}", cell.escaped_csv_string(), delim)
                    }
                } else {
                    delim.to_string()
                };
                write!(f, "{data}")?;
            }
        }
        f.flush()?;

        self.dirty = false;
        Ok(())
    }

    pub fn get_grid<'a>(&'a self) -> &'a CellGrid {
        &self.grid_history[self.current_grid]
    }

    fn get_grid_mut<'a>(&'a mut self) -> &'a mut CellGrid {
        &mut self.grid_history[self.current_grid]
    }

    fn transact_on_grid<F>(&mut self, mut action: F)
    where
        F: FnMut(&mut CellGrid) -> (),
    {
        let old = self.get_grid().clone();
        self.grid_history.push(old);
        action(&mut self.grid_history[self.current_grid]);

        self.dirty = true;
    }

    pub fn needs_to_be_saved(&self) -> bool {
        self.dirty
    }

    fn parse_csv_line(line: &str) -> Vec<Option<String>> {
        let mut iter = line.as_bytes().iter().map(|f| *f as char).peekable();
        let mut cells = Vec::new();
        let mut token = Vec::new();

        let mut inside_quotes = false;
        let mut is_escaped = false;

        while let Some(c) = iter.next() {
            // we just finished
            if c == CSV_DELIMITER && !inside_quotes {
                if !token.is_empty() {
                    cells.push(Some(token.iter().collect::<String>()));
                } else {
                    cells.push(None);
                }
                token.clear();
                continue;
            }
            // start reading an escaped cell
            if c == '"' {
                if inside_quotes {
                    // we might be escaping a quote
                    if let Some(next) = iter.peek() {
                        // check if the next cell is a quote, if it is, that's because it's being escaped by the current quote
                        // only escape the next char if this char isn't escaped it's self
                        if *next == '"' && !is_escaped {
                            // don't save the escape char
                            is_escaped = true;
                            continue;
                        } else if is_escaped {
                            is_escaped = false;
                        } else {
                            // escaped cell over
                            inside_quotes = false;
                            continue;
                        }
                    } else {
                        if is_escaped {
                            is_escaped = false;
                        } else {
                            continue;
                        }
                    }
                } else {
                    // not inside quotes, must be escaping another one
                    if let Some(next) = iter.peek() {
                        if *next == '"' && !is_escaped {
                            // the current char is " and the next char is "
                            // forget this one and mark to save the next
                            is_escaped = true;
                            continue;
                        } else if is_escaped {
                            is_escaped = false;
                        } else {
                            inside_quotes = true;
                            continue;
                        }
                    } else {
                        // not inside quotes, EOL

                        if is_escaped {
                            is_escaped = false;
                        }
                    }
                }
            }
            token.push(c)
        }
        if !token.is_empty() {
            cells.push(Some(token.iter().collect::<String>()));
        }
        cells
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.selected_cell
    }

    pub fn mv_cursor_to(&mut self, x: usize, y: usize) {
        self.selected_cell = (x, y)
    }

    pub fn apply_momentum(&mut self, (x, y): (i32, i32)) {
        let (cx, cy) = self.cursor();

        assert_eq!(0i32.saturating_add(-1), -1);
        let x = (cx as i32).saturating_add(x);
        let y = (cy as i32).saturating_add(y);

        // make it positive
        let x = max(x, 0) as usize;
        let y = max(y, 0) as usize;

        // keep it in the grid
        let x = min(x, LEN - 1);
        let y = min(y, LEN - 1);

        self.mv_cursor_to(x, y);
    }

    pub fn insert_row_above(&mut self, (_x, insertion_y): (usize, usize)) {
        self.transact_on_grid(|grid: &mut CellGrid| {
            grid.insert_row(insertion_y);
            for x in 0..LEN {
                for y in 0..LEN {
                    if let Some(cell) = grid.get_cell_raw(x, y).as_ref().map(|f| {
                        f.custom_translate_cell((0, 0), (0, 1), |rolling, old, new| {
                            if let Some((_, arg_y)) = Grid::parse_to_idx(old) {
                                if arg_y < insertion_y { rolling.to_owned() } else { rolling.replace(old, new) }
                            } else {
                                unimplemented!("Invalid variable wanted to be translated")
                            }
                        })
                    }) {
                        grid.set_cell_raw((x, y), Some(cell));
                    }
                }
            }
        });
    }

    pub fn insert_row_below(&mut self, (x, y): (usize, usize)) {
        self.insert_row_above((x, y + 1));
    }

    pub fn insert_column_before(&mut self, (insertion_x, _y): (usize, usize)) {
        self.transact_on_grid(|grid| {
            grid.insert_column(insertion_x);
            for x in 0..LEN {
                for y in 0..LEN {
                    if let Some(cell) = grid.get_cell_raw(x, y).as_ref().map(|f| {
                        f.custom_translate_cell((0, 0), (1, 0), |rolling, old, new| {
                            if let Some((arg_x, _)) = Grid::parse_to_idx(old) {
                                // add 1 because of the insertion
                                if arg_x < insertion_x { rolling.to_owned() } else { rolling.replace(old, new) }
                            } else {
                                unimplemented!("Invalid variable wanted to be translated")
                            }
                        })
                    }) {
                        grid.set_cell_raw((x, y), Some(cell));
                    }
                }
            }
        });
    }

    pub fn insert_column_after(&mut self, (x, y): (usize, usize)) {
        self.insert_column_before((x + 1, y));
    }

    /// Only evaluates equations, such as `=10` or `=A1/C2`, not
    /// strings or numbers.
    pub fn evaluate(&self, mut eq: &str) -> Result<CellType, String> {
        if eq.starts_with('=') {
            eq = &eq[1..];
        } else {
            // Should be evaluating an equation
            return Err(format!("\"{eq}\" is not an equation"));
        }

        let ctx = ctx::CallbackContext::new(&self);

        let prep_for_return = |v: Value| {
            if v.is_number() {
                if v.is_float() {
                    let val = v.as_float().expect("Value lied about being a float");
                    return Ok(CellType::Number(val));
                } else if v.is_int() {
                    let val = v.as_int().expect("Value lied about being an int");
                    return Ok(CellType::Number(val as f64));
                }
            } else if v.is_string() {
                // ^^ This allows for functions to return a string
                let s = v.as_string().expect("Value lied about being a String");
                return Ok(CellType::String(s));
            }
            return Err("Result is NaN".to_string());
        };

        match eval_with_context(eq, &ctx) {
            Ok(e) => {
                return prep_for_return(e);
            }
            Err(e) => match e {
                EvalexprError::VariableIdentifierNotFound(var_not_found) => {
                    return Err(format!("\"{var_not_found}\" is not a variable"));
                }
                EvalexprError::TypeError { expected: e, actual: a } => {
                    // IE: You put a string into a function that wants a float
                    return Err(format!("Wanted {e:?}, got {a}"));
                }
                _ => return Err(e.to_string()),
            },
        }
    }

    pub fn char_to_idx(i: &str) -> usize {
        let x_idx = i
            .chars()
            .filter(|f| f.is_alphabetic())
            .enumerate()
            .map(|(idx, c)| ((c.to_ascii_lowercase() as usize).saturating_sub(97)) + (26 * idx))
            .fold(0, |a, b| a + b);
        x_idx
    }

    /// Parse values in the format of A0, C10 ZZ99, etc, and
    /// turn them into an X,Y index.
    pub fn parse_to_idx(i: &str) -> Option<(usize, usize)> {
        let i = i.replace('$', "");

        let chars = i.chars().take_while(|c| c.is_alphabetic()).collect::<String>();
        let nums = i.chars().skip(chars.len()).take_while(|c| c.is_numeric()).collect::<String>();

        // At least half the arguments are gone
        if chars.len() == 0 || nums.len() == 0 {
            return None;
        }

        // get the x index from the chars
        let x_idx = Self::char_to_idx(&chars);

        // get the y index from the numbers
        if let Ok(y_idx) = nums.parse::<usize>() {
            return Some((x_idx, y_idx));
        } else {
            return None;
        }
    }

    /// Helper for tests
    #[cfg(test)]
    /// Don't ever remove this from being just a test-helper.
    /// This function doesn't correctly use the undo/redo api, which would require doing
    /// transactions on the grid instead of direct access.
    pub fn set_cell<T: Into<CellType>>(&mut self, cell_id: &str, val: T) {
        if let Some(loc) = Self::parse_to_idx(cell_id) {
            self.set_cell_raw(loc, Some(val));
        }
        self.dirty = true;
    }

    #[deprecated]
    /// You should get the grid then transact on the grid it's self
    pub fn set_cell_raw<T: Into<CellType>>(&mut self, (x, y): (usize, usize), val: Option<T>) {
        // TODO check oob
        self.get_grid_mut().set_cell_raw((x,y), val.map(|v| v.into()));
        self.dirty = true;
    }

    /// Get cells via text like:
    /// A6,
    /// F0,
    /// etc
    pub fn get_cell(&self, cell_id: &str) -> &Option<CellType> {
        if let Some((x, y)) = Self::parse_to_idx(cell_id) {
            return self.get_cell_raw(x, y);
        }
        &None
    }

    pub fn get_cell_raw(&self, x: usize, y: usize) -> &Option<CellType> {
        if x >= LEN || y >= LEN {
            return &None;
        }
        &self.get_grid().get_cell_raw(x,y)
    }

    pub fn num_to_char(idx: usize) -> String {
        /*
          A = 0
         AA = 26
        AAA = Not going to worry about it yet
         */

        let mut word: [char; 2] = [' '; 2];

        if idx >= 26 {
            word[0] = ((idx / 26) + 65 - 1) as u8 as char;
        }
        word[1] = ((idx % 26) + 65) as u8 as char;

        word.iter().collect()
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn saving_csv() {
    // setup grid
    // This should be 1..10 in the A column, then
    // 1^2..10^2 in the B column.
    let mut app = App::new();
    app.grid.set_cell("A0", 1.);
    app.grid.set_cell("B0", "=A0^2".to_string());

    app.grid.set_cell("A1", "=A0+A$0".to_string());
    app.grid.set_cell("B1", "=A1^2".to_string());

    app.grid.mv_cursor_to(0, 1);
    app.mode = Mode::Visual(app.grid.cursor());
    Mode::process_key(&mut app, 'l');
    Mode::process_key(&mut app, 'y');
    app.mode = Mode::Normal;
    app.grid.mv_cursor_to(0, 2);
    for _ in 0..10 {
        Mode::process_key(&mut app, 'p');
    }
    // setup done

    // insure that the cells are there
    let cell = app.grid.get_cell_raw(0, 10).as_ref().expect("Should've been set");
    let res = app.grid.evaluate(&cell.to_string()).expect("Should evaluate");
    assert_eq!(res, (11.0).into());
    assert_eq!(cell.escaped_csv_string(), "=A9+A$0");
    let cell = app.grid.get_cell_raw(1, 10).as_ref().expect("Should've been set");
    let res = app.grid.evaluate(&cell.to_string()).expect("Should evaluate");
    assert_eq!(res, (121.0).into());
    assert_eq!(cell.escaped_csv_string(), "=A10^2");

    // set saving the file
    let filename = "/tmp/file.csv";
    app.grid.save_to(filename).expect("This will only work on linux systems");
    let mut file = fs::OpenOptions::new().read(true).open(filename).expect("Just wrote the file");
    let mut buf = String::new();
    file.read_to_string(&mut buf).expect("Just opened the file");
    let line = buf.lines().skip(10).next();
    assert_eq!(line, Some("11,121"));
}

#[test]
fn saving_neoscim() {
    // setup grid
    // This should be 1..10 in the A column, then
    // 1^2..10^2 in the B column.
    let mut app = App::new();
    app.grid.set_cell("A0", 1.);
    app.grid.set_cell("B0", "=A0^2".to_string());

    app.grid.set_cell("A1", "=A0+A$0".to_string());
    app.grid.set_cell("B1", "=A1^2".to_string());

    app.grid.mv_cursor_to(0, 1);
    app.mode = Mode::Visual(app.grid.cursor());
    Mode::process_key(&mut app, 'l');
    Mode::process_key(&mut app, 'y');
    app.mode = Mode::Normal;
    app.grid.mv_cursor_to(0, 2);
    for _ in 0..10 {
        Mode::process_key(&mut app, 'p');
    }
    // setup done

    // insure that the cells are there
    let cell = app.grid.get_cell_raw(0, 10).as_ref().expect("Should've been set");
    let res = app.grid.evaluate(&cell.to_string()).expect("Should evaluate");
    assert_eq!(res, (11.0).into());
    assert_eq!(cell.escaped_csv_string(), "=A9+A$0");
    let cell = app.grid.get_cell_raw(1, 10).as_ref().expect("Should've been set");
    let res = app.grid.evaluate(&cell.to_string()).expect("Should evaluate");
    assert_eq!(res, (121.0).into());
    assert_eq!(cell.escaped_csv_string(), "=A10^2");

    // set saving the file
    let filename = format!("/tmp/file.{CUSTOM_EXT}");
    app.grid.save_to(&filename).expect("This will only work on linux systems");
    let mut file = fs::OpenOptions::new().read(true).open(filename).expect("Just wrote the file");
    let mut buf = String::new();
    file.read_to_string(&mut buf).expect("Just opened the file");
    let line = buf.lines().skip(10).next();
    assert_eq!(line, Some("=A9+A$0,=A10^2"));
}

// Do cells hold strings?
#[test]
fn cell_strings() {
    let mut grid = Grid::new();

    assert!(&grid.get_grid().get_cell_raw(0,0).is_none());
    grid.set_cell("A0", "Hello".to_string());
    assert!(grid.get_cell("A0").is_some());

    assert_eq!(grid.get_cell("A0").as_ref().unwrap().to_string(), String::from("Hello"));
}

// Testing if A0 -> 0,0 and if 0,0 -> A0
#[test]
fn alphanumeric_indexing() {
    assert_eq!(Grid::parse_to_idx("A0"), Some((0, 0)));
    assert_eq!(Grid::parse_to_idx("$A0"), Some((0, 0)));
    assert_eq!(Grid::parse_to_idx("A$0"), Some((0, 0)));
    assert_eq!(Grid::parse_to_idx("$A$0"), Some((0, 0)));
    assert_eq!(Grid::parse_to_idx("AA0"), Some((26, 0)));
    assert_eq!(Grid::parse_to_idx("A1"), Some((0, 1)));
    assert_eq!(Grid::parse_to_idx("A10"), Some((0, 10)));
    assert_eq!(Grid::parse_to_idx("Aa10"), Some((26, 10)));
    assert_eq!(Grid::parse_to_idx("invalid"), None);
    assert_eq!(Grid::parse_to_idx("1"), None);
    assert_eq!(Grid::parse_to_idx("A"), None);
    assert_eq!(Grid::parse_to_idx(":"), None);
    assert_eq!(Grid::parse_to_idx("="), None);
    assert_eq!(Grid::parse_to_idx("A:A"), None);

    assert_eq!(Grid::num_to_char(0).trim(), "A");
    assert_eq!(Grid::num_to_char(25).trim(), "Z");
    assert_eq!(Grid::num_to_char(26), "AA");
    assert_eq!(Grid::num_to_char(51), "AZ");
    assert_eq!(Grid::num_to_char(701), "ZZ");
    // Larger than ZZ isn't implemented yet
}

#[test]
fn valid_equations() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 2.);
    grid.set_cell("B0", 1.);
    grid.set_cell("C0", "=A0+B0".to_string());

    // cell math
    let cell = grid.get_cell("C0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, (3.).into());

    // divide floats
    grid.set_cell("D0", "=5./2.".to_string());
    let cell = grid.get_cell("D0").as_ref().expect("I just set this");
    let res = grid.evaluate(&cell.to_string()).expect("Should be ok");
    assert_eq!(res, (2.5).into());

    // Float / Int mix
    grid.set_cell("D0", "=5./2".to_string());
    let cell = grid.get_cell("D0").as_ref().expect("I just set this");
    let res = grid.evaluate(&cell.to_string()).expect("Should be ok");
    assert_eq!(res, (2.5).into());

    // divide "ints" (should become floats)
    grid.set_cell("D0", "=5/2".to_string());
    let cell = grid.get_cell("D0").as_ref().expect("I just set this");
    let res = grid.evaluate(&cell.to_string()).expect("Should be ok");
    assert_eq!(res, (2.5).into());

    // Non-equation that should still be valid
    grid.set_cell("D0", "=10".to_string());
    let cell = grid.get_cell("D0").as_ref().expect("I just set this");
    let res = grid.evaluate(&cell.to_string()).expect("Should be ok");
    assert_eq!(res, (10.).into());
}

// Cell = output of Cell = value of Cells.
// Cell who's value depends on the output of another cell's equation.
#[test]
fn fn_of_fn() {
    let mut grid = Grid::new();
    grid.set_cell("A0", 2.);
    grid.set_cell("B0", 1.);
    grid.set_cell("C0", "=A0+B0".to_string());
    grid.set_cell("D0", "=C0*2".to_string());

    if let Some(cell) = grid.get_cell("D0") {
        let res = grid.evaluate(&cell.to_string());

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (6.).into());
        return;
    }
    panic!("Cell not found");
}

// Two cells that have a circular dependency to solve for a value
#[test]
fn circular_reference_cells() {
    let mut grid = Grid::new();
    grid.set_cell("A0", "=B0".to_string());
    grid.set_cell("B0", "=A0".to_string());

    if let Some(cell) = grid.get_cell("A0") {
        let res = grid.evaluate(&cell.to_string());
        assert!(res.is_err());
        return;
    }
    panic!("Cell not found");
}

#[test]
fn invalid_equations() {
    let mut grid = Grid::new();

    // Test invalidly formatted equation
    grid.set_cell("A0", "=invalid".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_err());

    // Test an "equation" that's just 1 number
    grid.set_cell("B0", "=10".to_string());
    let cell = grid.get_cell("B0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert!(res.is_ok_and(|v| v == (10.).into()));

    // Trailing comma in function call
    grid.set_cell("A0", 5.);
    grid.set_cell("A1", 10.);
    grid.set_cell("B0", "=avg(A0,A1,)".to_string());
    let cell = grid.get_cell("B0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert_eq!(res.unwrap(), (7.5).into());
}

#[test]
fn grid_max() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 1.);
    let (mx, my) = grid.get_grid().max();
    assert_eq!(mx, 0);
    assert_eq!(my, 0);

    grid.set_cell("B0", 1.);
    let (mx, my) = grid.get_grid().max();
    assert_eq!(mx, 1);
    assert_eq!(my, 0);

    grid.set_cell("B5", 1.);
    let (mx, my) = grid.get_grid().max();
    assert_eq!(mx, 1);
    assert_eq!(my, 5);
}

#[test]
fn avg_function() {
    let mut grid = Grid::new();

    grid.set_cell("A0", "=avg(5)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (5.).into());

    grid.set_cell("A0", "=avg(5,10)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (7.5).into());

    grid.set_cell("A0", "=avg(5,10,15)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (10.).into());

    grid.set_cell("A0", "=avg(foo)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_err());

    grid.set_cell("A0", "=avg(1, foo)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_err());

    grid.set_cell("A0", "=avg()".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_err());
}

#[test]
fn sum_function() {
    let mut grid = Grid::new();

    grid.set_cell("A0", "=sum(5)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (5.).into());

    grid.set_cell("A0", "=sum(5,10)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (15.).into());

    grid.set_cell("A0", "=sum(foo)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_err());

    grid.set_cell("A0", "=sum(1, foo)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_err());

    grid.set_cell("A0", "=sum()".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_err());
}

#[test]
fn xlookup_function() {
    let mut grid = Grid::new();

    grid.set_cell("A0", "Bobby".to_string());
    grid.set_cell("A1", "Sarah".to_string());
    grid.set_cell("C0", 31.);
    grid.set_cell("C1", 41.);
    grid.set_cell("B0", "=xlookup(\"Bobby\",A:A,C:C)".to_string());
    let cell = grid.get_cell("B0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (31.).into());
}

#[test]
fn parse_csv() {
    //standard parsing
    assert_eq!(
        Grid::parse_csv_line("1,2,3"),
        vec![Some("1".to_string()), Some("2".to_string()), Some("3".to_string())]
    );

    // comma in a cell
    assert_eq!(
        Grid::parse_csv_line("1,\",\",3"),
        vec![Some("1".to_string()), Some(",".to_string()), Some("3".to_string())]
    );

    // quotes in a cell
    assert_eq!(
        Grid::parse_csv_line("1,she said \"\"wow\"\",3"),
        vec![Some("1".to_string()), Some("she said \"wow\"".to_string()), Some("3".to_string())]
    );

    // quotes and comma in cell
    assert_eq!(
        Grid::parse_csv_line("1,\"she said \"\"hello, world\"\"\",3"),
        vec![Some("1".to_string()), Some("she said \"hello, world\"".to_string()), Some("3".to_string())]
    );

    // ending with a quote
    assert_eq!(
        Grid::parse_csv_line("1,she said \"\"hello world\"\""),
        vec![Some("1".to_string()), Some("she said \"hello world\"".to_string())]
    );

    // ending with a quote with a comma
    assert_eq!(
        Grid::parse_csv_line("1,\"she said \"\"hello, world\"\"\""),
        vec![Some("1".to_string()), Some("she said \"hello, world\"".to_string())]
    );

    // starting with a quote
    assert_eq!(
        Grid::parse_csv_line("\"\"hello world\"\" is what she said,1"),
        vec![Some("\"hello world\" is what she said".to_string()), Some("1".to_string())]
    );

    // starting with a quote with a comma
    assert_eq!(
        Grid::parse_csv_line("\"\"\"hello, world\"\" is what she said\",1"),
        vec![Some("\"hello, world\" is what she said".to_string()), Some("1".to_string())]
    );
}

#[test]
fn invalid_ranges() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 2.);
    grid.set_cell("A1", 1.);
    // ASCII to number conversion needs to not overflow
    grid.set_cell("B0", "=sum($:A)".to_string());

    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    let _ = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
}

#[test]
fn ranges() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 2.);
    grid.set_cell("A1", 1.);
    grid.set_cell("B0", "=sum(A:A)".to_string());

    // range with numbers
    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, (3.).into());

    // use range output as input for other function
    grid.set_cell("B1", "=B0*2".to_string());
    let cell = grid.get_cell("B1").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, (6.).into());

    // use equation outputs as range input
    grid.set_cell("A2", "=C0+1".to_string());
    grid.set_cell("C0", 5.);

    let cell = grid.get_cell("A2").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, (6.).into());

    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, (9.).into());

    // use function outputs as range input
    grid.set_cell("B1", 2.);
    grid.set_cell("C0", "=sum(B:B)".to_string());
    grid.set_cell("A2", "null".to_string());

    let cell = grid.get_cell("C0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, (5.).into());

    // use range outputs as range input
    grid.set_cell("D0", "=sum(C:C)".to_string());
    grid.set_cell("C1", 1.);

    let cell = grid.get_cell("D0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, (6.).into());
}

#[test]
fn recursive_ranges() {
    let mut grid = Grid::new();

    grid.set_cell("A1", 1.);
    grid.set_cell("B1", 2.);
    grid.set_cell("B0", "=sum(A:A)".to_string());
    grid.set_cell("A0", "=sum(B:B)".to_string());

    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_err());
}

#[test]
fn cursor_fns() {
    // surprisingly, this test was needed
    let mut app = App::new();
    let c = app.grid.cursor();
    assert_eq!(c, (0, 0));

    app.grid.mv_cursor_to(1, 0);
    assert_eq!(app.grid.cursor(), (1, 0));
}

#[test]
fn insert_col_before_1() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 2.);
    grid.set_cell("B0", "=A0*2".to_string());
    grid.set_cell("B1", "=B0".to_string());

    grid.mv_cursor_to(1, 0);

    grid.insert_column_before(grid.cursor());

    // cell didn't get translated
    let cell = grid.get_cell("A0").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "2");

    // cell referencing another cell on the same side of the insertion
    let cell = grid.get_cell("C1").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=C0");
}

#[test]
fn insert_col_before_2() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 2.);
    grid.set_cell("B0", "=A0*2".to_string());
    grid.set_cell("B1", "=B0".to_string());

    grid.mv_cursor_to(0, 0);

    grid.insert_column_before(grid.cursor());

    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "2");

    // cell referencing another cell on the same side of the insertion
    let cell = grid.get_cell("C1").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=C0");
}

#[test]
fn insert_col_before_3() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 2.);
    grid.set_cell("A1", 2.);
    grid.set_cell("B0", "=A0*A1".to_string());

    grid.mv_cursor_to(0, 0);

    grid.insert_column_before(grid.cursor());

    let cell = grid.get_cell("C0").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=B0*B1");
}

#[test]
fn insert_row_above_1() {
    let mut grid = Grid::new();

    grid.set_cell("A1", 2.);
    grid.set_cell("A0", "=A1*2".to_string());
    grid.set_cell("B0", "=A0".to_string());

    // shift half down
    grid.mv_cursor_to(0, 1);
    grid.insert_row_above(grid.cursor());

    // cell didn't get translated
    let cell = grid.get_cell("A0").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=A2*2");

    // cell referencing another cell on the same side of the insertion
    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=A0");
}

#[test]
fn insert_row_above_2() {
    let mut grid = Grid::new();

    grid.set_cell("A1", 2.);
    grid.set_cell("A0", "=A1*2".to_string());
    grid.set_cell("B0", "=A0".to_string());

    // shift nothing down
    grid.mv_cursor_to(0, 2);
    grid.insert_row_above(grid.cursor());

    // cell didn't get translated
    let cell = grid.get_cell("A0").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=A1*2");

    // cell referencing another cell on the same side of the insertion
    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=A0");
}

#[test]
fn insert_row_above_3() {
    let mut grid = Grid::new();

    grid.set_cell("A1", 2.);
    grid.set_cell("A0", "=A1*2".to_string());
    grid.set_cell("B0", "=A0".to_string());

    // shift everything down
    grid.mv_cursor_to(0, 0);
    grid.insert_row_above(grid.cursor());

    // cell didn't get translated
    let cell = grid.get_cell("A1").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=A2*2");

    // cell referencing another cell on the same side of the insertion
    let cell = grid.get_cell("B1").as_ref().expect("Just set it");
    assert_eq!(cell.to_string(), "=A1");
}

#[test]
fn cell_eval_depth() {
    use crate::app::mode::*;
    let mut app = App::new();

    app.grid.set_cell("A0", 1.);
    app.grid.set_cell("A1", "=A0+$A$0".to_string());

    app.grid.mv_cursor_to(0, 1);
    app.mode = Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');
    Mode::process_key(&mut app, 'j');
    app.mode = Mode::Chord(Chord::new('5'));
    Mode::process_key(&mut app, 'p');

    assert_eq!(app.grid.cursor(), (0, 7));

    let c = app.grid.get_cell("A6").as_ref().expect("Just set it");

    assert_eq!(c.to_string(), "=A5+$A$0");

    let res = app.grid.evaluate(&c.to_string()).expect("Should evaluate");
    assert_eq!(res, (7.).into());
}

#[test]
fn return_string_from_fn() {
    let mut grid = Grid::new();
    grid.set_cell("A0", "=if(2>1, \"A\", \"B\")".to_string()); // true, A
    grid.set_cell("A1", "=if(1>2, \"A\", \"B\")".to_string()); // false, B

    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    if let Ok(f) = res {
        if let CellType::String(s) = f {
            assert_eq!("A", s);
        } else {
            unreachable!();
        }
    }

    let cell = grid.get_cell("A1").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    if let Ok(f) = res {
        if let CellType::String(s) = f {
            assert_eq!("B", s);
        } else {
            unreachable!();
        }
    }
}
