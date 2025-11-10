mod app;
mod ctx;

use crate::app::{app::App, calc::{Grid}};

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
    app.grid.set_cell("A0", "Apples".to_string());
    app.grid.set_cell("A1", 10.);
    app.grid.set_cell("B0", "Bananas".to_string());
    app.grid.set_cell("B1", 10.);
    app.grid.set_cell("C0", "Fruit".to_string());
    app.grid.set_cell("C1", "=A1+B1".to_string());

    let res = app.run(term);
    ratatui::restore();
    return res;
}
