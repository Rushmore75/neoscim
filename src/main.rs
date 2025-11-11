mod app;

use crate::app::{app::App};

fn main() -> Result<(), std::io::Error> {
    let term = ratatui::init();
    let mut app = App::new();
    app.grid.set_cell("A0", "Apples".to_string());
    app.grid.set_cell("A1", 10.);
    app.grid.set_cell("B0", "Bananas".to_string());
    app.grid.set_cell("B1", 10.);
    app.grid.set_cell("C0", "Fruit".to_string());
    app.grid.set_cell("C1", "=A1+B1".to_string());

    app.grid.set_cell("D0", "x2".to_string());
    app.grid.set_cell("D1", "=C1*2".to_string());

    app.grid.set_cell("A4", "Recursive references don't break anything!".to_string());
    app.grid.set_cell("A5", "=B5".to_string());
    app.grid.set_cell("B5", "=A5".to_string());

    let res = app.run(term);
    ratatui::restore();
    return res;
}
