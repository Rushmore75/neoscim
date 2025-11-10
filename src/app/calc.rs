use std::fmt::Display;

use evalexpr::*;

use crate::ctx;

// if this is very large at all it will overflow the stack
pub const LEN: usize = 100;

pub struct Grid {
    //  a b c ...
    // 0
    // 1
    // 2
    // ...
    cells: [[Option<Box<dyn Cell>>; LEN]; LEN],
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
        // TODO this needs to be moved to the heap
        let b: [[Option<Box<dyn Cell>>; LEN]; LEN] =
            core::array::from_fn(|_| core::array::from_fn(|_| None));

        Self {
            cells: b,
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

    pub fn set_cell<T: Into<Box<dyn Cell>>>(&mut self, cell_id: &str, val: T) {
        let loc = Self::parse_to_idx(cell_id);
        self.set_cell_raw(loc, val);
    }

    pub fn set_cell_raw<T: Into<Box<dyn Cell>>>(&mut self, (x,y): (usize, usize), val: T) {
        // TODO check oob
        self.cells[x][y] = Some(val.into());
    }

    /// Get cells via text like:
    /// A6,
    /// F0,
    /// etc
    pub fn get_cell(&self, cell_id: &str) -> &Option<Box<dyn Cell>> {
        let (x, y) = Self::parse_to_idx(cell_id);
        self.get_cell_raw(x, y)
    }

    pub fn get_cell_raw(&self, x: usize, y: usize) -> &Option<Box<dyn Cell>> {
        // TODO check oob
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

pub trait Cell {
    /// Important! This is IS NOT the return value of an equation.
    /// This is the raw equation it's self.
    fn as_raw_string(&self) -> String;
    fn can_be_number(&self) -> bool;
    fn as_num(&self) -> f64;
    fn is_equation(&self) -> bool {
        self.as_raw_string().starts_with('=')
    }
}

impl Display for dyn Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        let disp = if self.can_be_number() {
            self.as_num().to_string()
        } else {
            self.as_raw_string()
        };

        write!(f, "{disp}")
    }
}

impl Cell for f64 {
    fn as_raw_string(&self) -> String {
        ToString::to_string(self)
    }

    fn can_be_number(&self) -> bool {
        true
    }

    fn as_num(&self) -> f64 {
        *self
    }
}

impl Into<Box<dyn Cell>> for f64 {
    fn into(self) -> Box<dyn Cell> {
        Box::new(self)
    }
}

impl Into<Box<dyn Cell>> for String {
    fn into(self) -> Box<dyn Cell> {
        Box::new(self)
    }
}

impl Cell for String {
    fn as_raw_string(&self) -> String {
        ToString::to_string(self)
    }

    fn can_be_number(&self) -> bool {
        // checking if the string is an equation
        self.starts_with('=')
    }

    fn as_num(&self) -> f64 {
        unimplemented!("&str cannot be used in a numeric context")
    }
}

#[test]
fn test_cells() {
    let mut grid = Grid::new();

    assert!(&grid.cells[0][0].is_none());
    grid.set_cell("A0", "Hello".to_string());
    assert!(grid.get_cell("A0").is_some());

    assert_eq!(
        grid.get_cell("A0").as_ref().unwrap().as_raw_string(),
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
