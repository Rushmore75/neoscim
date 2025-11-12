use std::fmt::Display;

use evalexpr::*;

use crate::app::logic::ctx;

pub const LEN: usize = 1000;


pub struct Grid {
    //  a b c ...
    // 0
    // 1
    // 2
    // ...
    cells: Vec<Vec<Option<CellType>>>,
    /// (X, Y)
    pub selected_cell: (usize, usize),
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("cells", &"Too many to print")
            .finish()
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
        }
    }

    /// Only evaluates equations, such as `=10` or `=A1/C2` not,
    /// strings or numbers.
    pub fn evaluate(&self, mut eq: &str) -> Option<f64> {
        if eq.starts_with('=') {
            eq = &eq[1..];
        } else {
            // Should be evaluating an equation
            return None
        }

        let ctx = ctx::CallbackContext::new(&self);

        match eval_with_context(eq, &ctx) {
            Ok(e) => {
                if e.is_number() {
                    if e.is_float() {
                        let val = e.as_float().expect("Value lied about being a float");
                        return Some(val);
                    } else if e.is_int() {
                        let i = e.as_int().expect("Value lied about being an int");
                        return Some(i as f64)
                    }
                }
                return None
            }
            Err(e) => match e {
                EvalexprError::VariableIdentifierNotFound(_e) => {
                    // panic!("Will not be able to parse this equation, cell {e} not found")
                    return None
                }
                _ => panic!("{}", e),
            },
        }
    }

    /// Parse values in the format of A0, C10 ZZ99, etc, and
    /// turn them into an X,Y index.
    fn parse_to_idx(i: &str) -> Option<(usize, usize)> {
        let chars = i
            .chars()
            .take_while(|c| c.is_alphabetic())
            .collect::<Vec<char>>();
        let nums = i
            .chars()
            .skip(chars.len())
            .take_while(|c| c.is_numeric())
            .collect::<String>();

        // get the x index from the chars
        let x_idx = chars
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                // convert to numbers
                (c.to_ascii_lowercase() as usize - 97) + (26 * idx)
            })
            .fold(0, |a, b| a + b);

        // get the y index from the numbers
        if let Ok(y_idx) = nums.parse::<usize>() {
            return Some((x_idx, y_idx));
        } else {
            return None
        }

    }

    pub fn set_cell<T: Into<CellType>>(&mut self, cell_id: &str, val: T) {
        if let Some(loc) = Self::parse_to_idx(cell_id) {
            self.set_cell_raw(loc, val);
        }
    }

    pub fn set_cell_raw<T: Into<CellType>>(&mut self, (x,y): (usize, usize), val: T) {
        // TODO check oob
        self.cells[x][y] = Some(val.into());
    }

    /// Get cells via text like:
    /// A6,
    /// F0,
    /// etc
    pub fn get_cell(&self, cell_id: &str) -> &Option<CellType> {
        if let Some((x, y)) = Self::parse_to_idx(cell_id) {
            return self.get_cell_raw(x, y)
        }
        &None
    }

    pub fn get_cell_raw(&self, x: usize, y: usize) -> &Option<CellType> {
        if x >= LEN || y >= LEN {
            return &None
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
            word[0]= ((idx/26) + 65 -1) as u8 as char;
        }
        word[1]= ((idx%26) + 65) as u8 as char;

        word.iter().collect()
    }

}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

pub enum CellType {
    Number(f64),
    String(String),
    Equation(String),
}

impl Into<CellType> for f64 {
    fn into(self) -> CellType {
        CellType::duck_type(self.to_string())
    }
}

impl Into<CellType> for String {
    fn into(self) -> CellType {
        CellType::duck_type(self)
    }
}

impl CellType {
    fn duck_type<'a>(value: impl Into<String>) -> Self {
        let value = value.into();

        if let Ok(parse) = value.parse::<f64>() {
            Self::Number(parse)
        } else {
            if value.starts_with('=') {
                Self::Equation(value)
            } else {
                Self::String(value)
            }
        }
    }
}

impl Display for CellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = match self {
            CellType::Number(n) => n.to_string(),
            CellType::String(n) => n.to_owned(),
            CellType::Equation(r) => r.to_owned(),
        };
        write!(f, "{d}")
    }
}

// Do cells hold strings?
#[test]
fn cell_strings() {
    let mut grid = Grid::new();

    assert!(&grid.cells[0][0].is_none());
    grid.set_cell("A0", "Hello".to_string());
    assert!(grid.get_cell("A0").is_some());

    assert_eq!(
        grid.get_cell("A0").as_ref().unwrap().to_string(),
        String::from("Hello")
    );
}

// Testing if A0 -> 0,0 and if 0,0 -> A0
#[test]
fn alphanumeric_indexing() {
    assert_eq!(Grid::parse_to_idx("A0"), Some((0, 0)));
    assert_eq!(Grid::parse_to_idx("AA0"), Some((26, 0)));
    assert_eq!(Grid::parse_to_idx("A1"), Some((0, 1)));
    assert_eq!(Grid::parse_to_idx("A10"), Some((0, 10)));
    assert_eq!(Grid::parse_to_idx("Aa10"), Some((26, 10)));
    assert_eq!(Grid::parse_to_idx("invalid"), None);
    
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

        assert!(res.is_some());
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
        assert!(res.is_none());
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
    assert!(res.is_none());

    // Test an "equation" that's just 1 number
    grid.set_cell("B0", "=10".to_string());
    let cell = grid.get_cell("B0").as_ref().expect("Just set the cell");
    let res = grid.evaluate(&cell.to_string());
    assert!(res.is_some());
    assert!(res.is_some_and(|v| v == 10.));
    
}