use std::fmt::Display;

use evalexpr::*;

use crate::app::ctx;

pub const LEN: usize = 100;

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

    pub fn evaluate(&self, mut eq: &str) -> Option<f64> {
        if eq.starts_with('=') {
            eq = &eq[1..];
        } else {
            // Should be evaluating an equation
            return None
        }

        let ctx = ctx::CallbackContext::new(&self);
        // let mut ctx = HashMapContext::<DefaultNumericTypes>::new();

        match eval_with_context(eq, &ctx) {
            Ok(e) => {
                let val = e.as_float().expect("Should be float");
                return Some(val);
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

    fn parse_to_idx(i: &str) -> (usize, usize) {
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
            .map(Self::char_to_idx)
            .fold(0, |a, b| a + b);

        // get the y index from the numbers
        let y_idx = nums
            .parse::<usize>()
            .expect("Got non-number character after sorting for just numeric characters");

        (x_idx, y_idx)
    }

    pub fn set_cell<T: Into<CellType>>(&mut self, cell_id: &str, val: T) {
        let loc = Self::parse_to_idx(cell_id);
        self.set_cell_raw(loc, val);
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
        let (x, y) = Self::parse_to_idx(cell_id);
        self.get_cell_raw(x, y)
    }

    pub fn get_cell_raw(&self, x: usize, y: usize) -> &Option<CellType> {
        if x >= LEN || y >= LEN {
            return &None
        }
        &self.cells[x][y]
    }

    // this function has unit tests
    fn char_to_idx((idx, c): (usize, &char)) -> usize {
        (c.to_ascii_lowercase() as usize - 97) + 26 * idx
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
            CellType::Equation(r) => {
                r.to_owned()
            },
        };
        write!(f, "{d}")
    }
}

#[test]
fn test_cells() {
    let mut grid = Grid::new();

    assert!(&grid.cells[0][0].is_none());
    grid.set_cell("A0", "Hello".to_string());
    assert!(grid.get_cell("A0").is_some());

    assert_eq!(
        grid.get_cell("A0").as_ref().unwrap().to_string(),
        String::from("Hello")
    );
}

#[test]
fn c_to_i() {
    assert_eq!(Grid::char_to_idx((0, &'a')), 0);
    assert_eq!(Grid::char_to_idx((0, &'A')), 0);
    assert_eq!(Grid::char_to_idx((0, &'z')), 25);
    assert_eq!(Grid::char_to_idx((0, &'Z')), 25);
    assert_eq!(Grid::char_to_idx((1, &'a')), 26);

    assert_eq!(Grid::parse_to_idx("A0"), (0, 0));
    assert_eq!(Grid::parse_to_idx("AA0"), (26, 0));
    assert_eq!(Grid::parse_to_idx("A1"), (0, 1));
    assert_eq!(Grid::parse_to_idx("A10"), (0, 10));
    assert_eq!(Grid::parse_to_idx("Aa10"), (26, 10));
}

#[test]
fn i_to_c() {
    assert_eq!(Grid::num_to_char(0).trim(), "A");
    assert_eq!(Grid::num_to_char(25).trim(), "Z");
    assert_eq!(Grid::num_to_char(26), "AA");
    assert_eq!(Grid::num_to_char(51), "AZ");
    assert_eq!(Grid::num_to_char(701), "ZZ");
}

#[test]
fn test_math() {
    use evalexpr::*;

    let mut grid = Grid::new();
    grid.set_cell("A0", 2.);
    grid.set_cell("B0", 1.);
    grid.set_cell("C0", "=A0+B0".to_string());

    assert_eq!(eval("1+2").unwrap(), Value::Int(3));
    if let Some(cell) = grid.get_cell("C0") {
        match cell {
            CellType::Number(_) => todo!(),
            CellType::String(_) => todo!(),
            CellType::Equation(a) => {
                let res = grid.evaluate(&a);
                assert!(res.is_some());
                assert_eq!(res.unwrap(), 3.);
                return;
            },
        }
    }
    
    panic!("Should've found the value and returned");
}

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
fn fn_of_fn_one_shot() {
    let mut grid = Grid::new();
    grid.set_cell("A0", 2.);
    grid.set_cell("B0", 1.);
    grid.set_cell("C0", "=A0+B0".to_string());
    grid.set_cell("D0", "=C0*2".to_string());

    grid.set_cell("E0", "=D0+C0".to_string());

    if let Some(cell) = grid.get_cell("E0") {
        let res = grid.evaluate(&cell.to_string());

        assert!(res.is_some());
        assert_eq!(res.unwrap(), 9.);
        return;
    }
    panic!("Cell not found");
}

#[test]
fn cell_ref_string() {
    let mut grid = Grid::new();
    grid.set_cell("A0", 2.);
    grid.set_cell("B0", "=A0".to_string());

    if let Some(cell) = grid.get_cell("A0") {
        let res = grid.evaluate(&cell.to_string());

        assert!(res.is_none());
        return;
    }
    panic!("Cell not found");
}