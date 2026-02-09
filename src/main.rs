#![allow(clippy::needless_return)]
#![allow(clippy::len_zero)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::single_match)]

mod app;

use std::env::args;

use crate::app::{app::App};

fn main() -> Result<(), std::io::Error> {

    let args = args().collect::<Vec<String>>();
    let mut app = if args.len() > 1 {
        let file = &args[1];
        match App::new_with_file(file) {
            Ok(o) => o,
            Err(e) => {
                return Err(e);
            },
        }
    } else {
        App::new()
    };

    let term = ratatui::init();
    let res = app.run(term);
    ratatui::restore();
    return res;
}
