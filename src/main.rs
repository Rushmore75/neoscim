mod app;

use crate::app::{app::App};

fn main() -> Result<(), std::io::Error> {
    let term = ratatui::init();
    let mut app = App::new();
    let res = app.run(term);
    ratatui::restore();
    return res;
}
