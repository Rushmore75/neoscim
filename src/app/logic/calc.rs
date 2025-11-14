use std::{
    cmp::{max, min},
    fs,
    io::{Read, Write},
    path::PathBuf,
};

use evalexpr::*;

use crate::app::{
    logic::{
        cell::{CSV_DELIMITER, CellType},
        ctx,
    },
};

#[cfg(test)]
use crate::app::app::App;

pub const LEN: usize = 1000;
pub const CSV_EXT: &str = "csv";
pub const CUSTOM_EXT: &str = "nscim";

pub struct Grid {
    //  a b c ...
    // 0
    // 1
    // 2
    // ...
    cells: Vec<Vec<Option<CellType>>>,
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
        let mut a = Vec::with_capacity(LEN);
        for _ in 0..LEN {
            let mut b = Vec::with_capacity(LEN);
            for _ in 0..LEN {
                b.push(None)
            }
            a.push(b)
        }

        Self {
            cells: a,
            selected_cell: (0, 0),
            dirty: false,
        }
    }

    pub fn new_from_file(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let mut grid = Self::new();

        let mut file = fs::OpenOptions::new().read(true).open(path.into())?;
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
        let path = path.into();

        let resolve_values;

        match path.extension() {
            Some(ext) => match ext.to_str() {
                Some(CSV_EXT) => {
                    resolve_values = true;
                }
                Some(CUSTOM_EXT) => {
                    resolve_values = true;
                }
                _ => {
                    resolve_values = false;
                    // path.add_extension(CUSTOM_EXT);
                }
            },
            None => {
                resolve_values = false;
                // path.add_extension(CUSTOM_EXT);
            }
        }

        let mut f = fs::OpenOptions::new().write(true).append(false).truncate(true).create(true).open(path)?;
        let (mx, my) = self.max();
        for y in 0..=my {
            for x in 0..=mx {
                let cell = &self.cells[x][y];

                let data = if let Some(cell) = cell {
                    if let Ok(val) = self.evaluate(&cell.to_string())
                        && resolve_values
                    {
                        val.to_string()
                    } else {
                        cell.escaped_csv_string()
                    }
                } else {
                    CSV_DELIMITER.to_string()
                };

                write!(f, "{data}")?;
            }
            write!(f, "\n")?;
        }
        f.flush()?;

        self.dirty = false;
        Ok(())
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
        for x in 0..LEN {
            self.cells[x].insert(insertion_y, None);
            self.cells[x].pop();
            for y in 0..LEN {
                if let Some(cell) = self.get_cell_raw(x, y).as_ref().map(|f| {
                    f.custom_translate_cell((0, 0), (0, 1), |rolling, old, new| {
                        if let Some((_, arg_y)) = Grid::parse_to_idx(old) {
                            if arg_y < insertion_y { rolling.to_owned() } else { rolling.replace(old, new)  }
                        } else {
                            unimplemented!("Invalid variable wanted to be translated")
                        }
                    }
                    )}) {
                    self.set_cell_raw((x,y), Some(cell));
                } 
            }
        }
    }
    pub fn insert_row_below(&mut self, (x, y): (usize, usize)) {
        self.insert_row_above((x,y+1));
    }
    pub fn insert_column_before(&mut self, (insertion_x, _y): (usize, usize)) {
        let mut v = Vec::with_capacity(LEN);
        for _ in 0..LEN {
            v.push(None);
        }
        self.cells.insert(insertion_x, v);
        // keep the grid LEN
        self.cells.pop();
        for x in 0..LEN {
            for y in 0..LEN {
                if let Some(cell) = self.get_cell_raw(x, y).as_ref().map(|f| {
                    f.custom_translate_cell((0, 0), (1, 0), |rolling, old, new| {
                        if let Some((arg_x, _)) = Grid::parse_to_idx(old) {
                            // add 1 because of the insertion
                            if arg_x < insertion_x { rolling.to_owned() } else { rolling.replace(old, new) }
                        } else {
                            unimplemented!("Invalid variable wanted to be translated")
                        }
                    })
                }) {
                    self.set_cell_raw((x, y), Some(cell));
                }
            }
        }
    }
    pub fn insert_column_after(&mut self, (x, y): (usize, usize)) {
        self.insert_column_before((x + 1, y));
    }
    /// Iterate over the entire grid and see where
    /// the farthest modified cell is.
    #[must_use]
    fn max(&self) -> (usize, usize) {
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

    /// Only evaluates equations, such as `=10` or `=A1/C2`, not
    /// strings or numbers.
    pub fn evaluate(&self, mut eq: &str) -> Result<f64, String> {
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
                    return Ok(val);
                } else if v.is_int() {
                    let i = v.as_int().expect("Value lied about being an int");
                    return Ok(i as f64);
                }
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
                EvalexprError::TypeError {
                    expected: e,
                    actual: a,
                } => {
                    // IE: You put a string into a function that wants a float
                    return Err(format!("Wanted {e:?}, got {a}"));
                }
                _ => return Err(e.to_string()),
            },
        }
    }

    /// Parse values in the format of A0, C10 ZZ99, etc, and
    /// turn them into an X,Y index.
    pub fn parse_to_idx(i: &str) -> Option<(usize, usize)> {
        let i = i.replace('$', "");

        let chars = i.chars().take_while(|c| c.is_alphabetic()).collect::<Vec<char>>();
        let nums = i.chars().skip(chars.len()).take_while(|c| c.is_numeric()).collect::<String>();

        // At least half the arguments are gone
        if chars.len() == 0 || nums.len() == 0 {
            return None;
        }

        // get the x index from the chars
        let x_idx = chars
            .iter()
            .enumerate()
            .map(|(idx, c)| (c.to_ascii_lowercase() as usize - 97) + (26 * idx))
            .fold(0, |a, b| a + b);

        // get the y index from the numbers
        if let Ok(y_idx) = nums.parse::<usize>() {
            return Some((x_idx, y_idx));
        } else {
            return None;
        }
    }

    /// Helper for tests
    #[cfg(test)]
    pub fn set_cell<T: Into<CellType>>(&mut self, cell_id: &str, val: T) {
        if let Some(loc) = Self::parse_to_idx(cell_id) {
            self.set_cell_raw(loc, Some(val));
        }
    }

    pub fn set_cell_raw<T: Into<CellType>>(&mut self, (x, y): (usize, usize), val: Option<T>) {
        // TODO check oob
        self.cells[x][y] = val.map(|v| v.into());
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
        &self.cells[x][y]
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

// Do cells hold strings?
#[test]
fn cell_strings() {
    let mut grid = Grid::new();

    assert!(&grid.cells[0][0].is_none());
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
    assert_eq!(res, 3.);

    // divide floats
    grid.set_cell("D0", "=5./2.".to_string());
    let cell = grid.get_cell("D0").as_ref().expect("I just set this");
    let res = grid.evaluate(&cell.to_string()).expect("Should be ok");
    assert_eq!(res, 2.5);

    // Float / Int mix
    grid.set_cell("D0", "=5./2".to_string());
    let cell = grid.get_cell("D0").as_ref().expect("I just set this");
    let res = grid.evaluate(&cell.to_string()).expect("Should be ok");
    assert_eq!(res, 2.5);

    // divide "ints" (should become floats)
    grid.set_cell("D0", "=5/2".to_string());
    let cell = grid.get_cell("D0").as_ref().expect("I just set this");
    let res = grid.evaluate(&cell.to_string()).expect("Should be ok");
    assert_eq!(res, 2.5);

    // Non-equation that should still be valid
    grid.set_cell("D0", "=10".to_string());
    let cell = grid.get_cell("D0").as_ref().expect("I just set this");
    let res = grid.evaluate(&cell.to_string()).expect("Should be ok");
    assert_eq!(res, 10.);
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
        assert_eq!(res.unwrap(), 6.);
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
    assert!(res.is_ok_and(|v| v == 10.));

    // Trailing comma in function call
    grid.set_cell("A0", 5.);
    grid.set_cell("A1", 10.);
    grid.set_cell("B0", "=avg(A0,A1,)".to_string());
    let cell = grid.get_cell("B0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert_eq!(res.unwrap(), 7.5);
}

#[test]
fn grid_max() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 1.);
    let (mx, my) = grid.max();
    assert_eq!(mx, 0);
    assert_eq!(my, 0);

    grid.set_cell("B0", 1.);
    let (mx, my) = grid.max();
    assert_eq!(mx, 1);
    assert_eq!(my, 0);

    grid.set_cell("B5", 1.);
    let (mx, my) = grid.max();
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
    assert_eq!(res.unwrap(), 5.);

    grid.set_cell("A0", "=avg(5,10)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 7.5);

    grid.set_cell("A0", "=avg(5,10,15)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 10.);

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
    assert_eq!(res.unwrap(), 5.);

    grid.set_cell("A0", "=sum(5,10)".to_string());
    let cell = grid.get_cell("A0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 15.);

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
    grid.set_cell("B0", "=xlookup(A:A,\"Bobby\",C:C)".to_string());
    let cell = grid.get_cell("B0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 31.);
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
fn ranges() {
    let mut grid = Grid::new();

    grid.set_cell("A0", 2.);
    grid.set_cell("A1", 1.);
    grid.set_cell("B0", "=sum(A:A)".to_string());

    // range with numbers
    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, 3.);

    // use range output as input for other function
    grid.set_cell("B1", "=B0*2".to_string());
    let cell = grid.get_cell("B1").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, 6.);

    // use equation outputs as range input
    grid.set_cell("A2", "=C0+1".to_string());
    grid.set_cell("C0", 5.);

    let cell = grid.get_cell("A2").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, 6.);

    let cell = grid.get_cell("B0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, 9.);

    // use function outputs as range input
    grid.set_cell("B1", 2.);
    grid.set_cell("C0", "=sum(B:B)".to_string());
    grid.set_cell("A2", "null".to_string());

    let cell = grid.get_cell("C0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, 5.);

    // use range outputs as range input
    grid.set_cell("D0", "=sum(C:C)".to_string());
    grid.set_cell("C1", 1.);

    let cell = grid.get_cell("D0").as_ref().expect("Just set it");
    let res = grid.evaluate(&cell.to_string()).expect("Should evaluate.");
    assert_eq!(res, 6.);
}

#[test]
fn recursive_ranges() {
    // recursive ranges causes weird behavior
    // todo!();
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
