use std::{collections::HashMap, sync::RwLock};

use ratatui::{layout::Rect, prelude};

use crate::app::calc::LEN;

pub struct ScreenSpace {
    /// This is measured in cells
    scroll: (usize, usize),
    default_cell_len: usize,
    default_cell_hight: usize,
    /// This is measured in chars
    last_seen_screen_size: RwLock<(usize, usize)>
}

impl ScreenSpace {
    pub fn new() -> Self {
        Self {
            scroll: (0, 0),
            default_cell_len: 10,
            default_cell_hight: 1,
            last_seen_screen_size: RwLock::new((0,0))
        }
    }

    pub fn scroll_based_on_cursor_location(&mut self, (cursor_x, cursor_y): (usize, usize), vars: &HashMap<String, String>) {
        if let Ok(screen_size) = self.last_seen_screen_size.read() {
            
            // screen seems to be 2 cells smaller than it should be
            let x_cells = (screen_size.0 / self.get_cell_width(vars) as usize)-2;

            let lower_x = self.scroll_x();
            let upper_x = self.scroll_x() + x_cells;

            if cursor_x<=lower_x {
                self.scroll.0 = self.scroll.0.saturating_sub(1);
            }
            if cursor_x > upper_x {
                self.scroll.0 = self.scroll.0.saturating_add(1);
            }

        }
    }

    pub fn scroll_x(&self) -> usize {
        self.scroll.0
    }
    pub fn y(&self) -> usize{
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

        let x_max =
            if area.width as usize / self.get_cell_width(vars) > LEN { LEN - 1 } else { area.width as usize / self.get_cell_width(vars)};
        let y_max =
            if area.height as usize / self.get_cell_height(vars) > LEN { LEN - 1 } else { area.height as usize / self.get_cell_height(vars)};
        (x_max as u16, y_max as u16)
    }
}

