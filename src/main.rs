mod ctx;

use std::rc::Rc;

use evalexpr::*;

// if this is very large at all it will overflow the stack
const LEN: usize = 100;

struct Grid {
    //  a b c ...
    // 0
    // 1
    // 2
    // ...
    cells: [[Option<Box<dyn Cell>>; LEN]; LEN],
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid").field("cells", &"Too many to print").finish()
    }
}

impl Grid {
    fn new() -> Self {
        let b: [[Option<Box<dyn Cell>>; LEN]; LEN] =
            core::array::from_fn(|_| core::array::from_fn(|_| None));

        Self { cells: b }
    }

    fn eval(&self, mut eq: &str) -> f64 {
        if eq.starts_with('=') {
            eq = &eq[1..];
        }

        let mut ctx = ctx::CallbackContext::<DefaultNumericTypes>::new(Rc::new(self));
        // let mut ctx = HashMapContext::<DefaultNumericTypes>::new();

        let val;
        loop {
            match eval_with_context(eq, &ctx) {
                Ok(e) => {
                    val = e.as_float().expect("Should be float");
                    break;
                }
                Err(e) => match e {
                    // TODO this is kinda a slow way to do this, the equation will get parsed
                    // multiple times. Might be good to modify the lib so that you can provide
                    // a callback for variables that are not found.
                    EvalexprError::VariableIdentifierNotFound(e) => {
                        panic!("Will not be able to parse this equation, cell {e} not found")
                    }
                    _ => panic!("{}", e),
                },
            }
        }
        val
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

    fn set_cell(&mut self, cell_id: &str, val: Box<dyn Cell>) {
        let (x, y) = Self::parse_to_idx(cell_id);
        // TODO check oob
        self.cells[x][y] = Some(val);
    }

    /// Get cells via text like:
    /// A6
    /// F0
    fn get_cell(&self, cell_id: &str) -> &Option<Box<dyn Cell>> {
        let (x, y) = Self::parse_to_idx(cell_id);
        // TODO check oob
        &self.cells[x][y]
    }

    // this function has unit tests
    fn char_to_idx((idx, c): (usize, &char)) -> usize {
        (c.to_ascii_lowercase() as usize - 97) + 26 * idx
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

trait Cell {
    fn to_string(&self) -> String;
    fn can_be_number(&self) -> bool;
    fn as_num(&self) -> f32;
    fn is_eq(&self) -> bool {
        self.to_string().starts_with('=')
    }
}

impl Cell for f32 {
    fn to_string(&self) -> String {
        ToString::to_string(self)
    }

    fn can_be_number(&self) -> bool {
        true
    }

    fn as_num(&self) -> f32 {
        *self
    }
}

impl Cell for &str {
    fn to_string(&self) -> String {
        ToString::to_string(self)
    }

    fn can_be_number(&self) -> bool {
        // checking if the string is an equation
        self.starts_with('=')
    }

    fn as_num(&self) -> f32 {
        unimplemented!("&str cannot be used in a numeric context")
    }
}

#[test]
fn test_math() {
    let mut grid = Grid::new();
    grid.set_cell("A0", Box::new(2.));
    grid.set_cell("B0", Box::new(1.));
    grid.set_cell("C0", Box::new("=A0+B0"));

    assert_eq!(eval("1+2").unwrap(), Value::Int(3));

    let disp = &grid.get_cell("C0");
    if let Some(inner) = disp {
        if inner.is_eq() {
            println!("{}", inner.to_string());
            let display = grid.eval(&inner.to_string());
            assert_eq!(display, 3.);
            return;
        }
    }
    panic!("Should've found the value and returned");
}

#[test]
fn test_cells() {
    let mut grid = Grid::new();

    assert!(&grid.cells[0][0].is_none());
    grid.set_cell("A0", Box::new("Hello"));
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

fn main() {
    println!("Only tests exist atm");
}
