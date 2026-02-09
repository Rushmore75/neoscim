use std::{cmp::min, collections::HashMap, sync::RwLock};

use ratatui::prelude;

use crate::app::logic::calc::LEN;

pub struct ScreenSpace {
    /// This is measured in cells.
    /// This is the top-left cell index.
    scroll: (usize, usize),
    /// In chars
    default_cell_len: usize,
    /// In chars
    default_cell_hight: usize,
    /// This is measured in chars
    last_seen_screen_size: RwLock<(usize, usize)>
}

impl ScreenSpace {
    pub fn new() -> Self {
        Self {
            scroll: (0, 0),
            default_cell_len: 9,
            default_cell_hight: 1,
            last_seen_screen_size: RwLock::new((0,0))
        }
    }

    pub fn center_x(&mut self, (cursor_x, _): (usize, usize), vars: &HashMap<String, String>) {
        let (x_cells, _) = self.get_screen_size(vars);
        let x_center = self.scroll_x() + (x_cells/2);

        let delta = cursor_x as isize - x_center as isize;
        self.scroll.0 = self.scroll.0.saturating_add_signed(delta);
    }

    pub fn center_y(&mut self, (_, cursor_y): (usize, usize), vars: &HashMap<String, String>) {
        let (_, y_cells) = self.get_screen_size(vars);
        let y_center = self.scroll_y() + (y_cells/2);

        let delta = cursor_y as isize - y_center as isize;
        self.scroll.1 = self.scroll.1.saturating_add_signed(delta);
    }
    
    /// In chars
    pub fn get_screen_size(&self, vars: &HashMap<String, String>) -> (usize, usize) {
        if let Ok(screen_size) = self.last_seen_screen_size.read() {
            // ======= X =======
            // screen seems to be 2 cells smaller than it should be
            // this is probably related to issue #6
            let x_cells = (screen_size.0 / self.get_cell_width(vars)) -2;
            // ======= Y =======
            // screen seems to be 2 cells smaller than it should be
            // this is probably related to issue #6
            let y_cells = (screen_size.1 / self.get_cell_height(vars)) -2;
            (x_cells,y_cells)
        } else {
            (0,0)
        }
    }

    pub fn scroll_based_on_cursor_location(&mut self, (cursor_x, cursor_y): (usize, usize), vars: &HashMap<String, String>) {
        let (x_cells, y_cells) = self.get_screen_size(vars);


        let lower_x = self.scroll_x();
        let upper_x = self.scroll_x() + x_cells;

        if cursor_x < lower_x {
            let delta = lower_x - cursor_x;
            self.scroll.0 = self.scroll.0.saturating_sub(delta);
        }
        if cursor_x > upper_x {
            let delta = cursor_x - upper_x;
            self.scroll.0 = self.scroll.0.saturating_add(delta);
        }


        let lower_y = self.scroll_y();
        let upper_y = self.scroll_y() + y_cells;

        if cursor_y < lower_y {
            let delta = lower_y - cursor_y;
            self.scroll.1 = self.scroll.1.saturating_sub(delta);
        }

        if cursor_y > upper_y {
            let delta = cursor_y - upper_y;
            self.scroll.1 = self.scroll.1.saturating_add(delta);
        }
    }

    pub fn scroll_x(&self) -> usize {
        self.scroll.0
    }

    pub fn scroll_y(&self) -> usize{
        self.scroll.1
    }

    pub fn get_cell_height(&self, vars: &HashMap<String, String>) -> usize {
        if let Some(h) = vars.get("height") {
            if let Ok(p) = h.parse::<usize>() {
                return p;
            }
        }
        return self.default_cell_hight
    }
    
    pub fn get_cell_width(&self, vars: &HashMap<String, String>) -> usize {
        if let Some(h) = vars.get("length") {
            if let Ok(p) = h.parse::<usize>() {
                return p;
            }
        }
        self.default_cell_len

    }

    pub fn how_many_cells_fit_in(&self, area: &prelude::Rect, vars: &HashMap<String, String>) -> (u16, u16) {
        if let Ok(mut l) = self.last_seen_screen_size.write() {
            l.0 = area.width as usize;
            l.1 = area.height as usize;
        }

        let width = (area.width as usize / self.get_cell_width(vars)) + 1;
        let height = area.height as usize / self.get_cell_height(vars);

        let x_max = min(LEN-1, width);
        let y_max = min(LEN-1, height);

        (x_max as u16, y_max as u16)
    }

}

#[test]
fn fit_cells() {
    use crate::App;
    let mut app = App::new();
    app.vars.insert("length".to_string(), 10.to_string());
    app.vars.insert("height".to_string(), 1.to_string());

    let (x,y) = app.screen.how_many_cells_fit_in(&prelude::Rect::new(0, 0, 181, 14), &app.vars);
    assert_eq!(x, 18);
    assert_eq!(y, 14);
}

#[test]
fn scroll() {
    use crate::App;
    let mut app = App::new();
    app.vars.insert("length".to_string(), 10.to_string());
    app.vars.insert("height".to_string(), 1.to_string());

    // We have to check how many cells fit, because screen learns the width
    // of the area by rumour here.
    let (x,y) = app.screen.how_many_cells_fit_in(&prelude::Rect::new(0, 0, 181, 14), &app.vars);
    assert_eq!(x, 18);
    assert_eq!(y, 14);

    // we aren't scrolled at all yet
    app.screen.scroll_based_on_cursor_location((0,0), &app.vars);
    assert_eq!(app.screen.scroll.0, 0);
    assert_eq!(app.screen.scroll.1, 0);

    // at the edge of visible, but shouldn't scroll yet
    app.screen.scroll_based_on_cursor_location((16,0), &app.vars);
    assert_eq!(app.screen.scroll.0, 0);
    assert_eq!(app.screen.scroll.1, 0);

    // scroll 1 right
    // Yes, you would think this shouldn't scroll yet `floor(181/10) = 18`
    // but for some reason we are missing 2 cells in each direction.
    app.screen.scroll_based_on_cursor_location((17,0), &app.vars);
    assert_eq!(app.screen.scroll.0, 1);
    assert_eq!(app.screen.scroll.1, 0);

    // cursor comes back, scroll should stay
    app.screen.scroll_based_on_cursor_location((16,0), &app.vars);
    assert_eq!(app.screen.scroll.0, 1);
    assert_eq!(app.screen.scroll.1, 0);

    // scroll left 
    app.screen.scroll_based_on_cursor_location((0,0), &app.vars);
    assert_eq!(app.screen.scroll.0, 0);
    assert_eq!(app.screen.scroll.1, 0);

    // now for y

    // we aren't scrolled at all yet
    app.screen.scroll_based_on_cursor_location((0,0), &app.vars);
    assert_eq!(app.screen.scroll.0, 0);
    assert_eq!(app.screen.scroll.1, 0);

    app.screen.scroll_based_on_cursor_location((0,12), &app.vars);
    assert_eq!(app.screen.scroll.0, 0);
    assert_eq!(app.screen.scroll.1, 0);

    app.screen.scroll_based_on_cursor_location((0,13), &app.vars);
    assert_eq!(app.screen.scroll.0, 0);
    assert_eq!(app.screen.scroll.1, 1);

    app.screen.scroll_based_on_cursor_location((0,12), &app.vars);
    assert_eq!(app.screen.scroll.0, 0);
    assert_eq!(app.screen.scroll.1, 1);

    app.screen.scroll_based_on_cursor_location((0,0), &app.vars);
    assert_eq!(app.screen.scroll.0, 0);
    assert_eq!(app.screen.scroll.1, 0);

}
