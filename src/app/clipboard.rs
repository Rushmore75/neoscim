use crate::app::logic::{calc::Grid, cell::CellType};

#[cfg(test)]
use crate::app::{
    app::App,
    mode::{Chord, Mode},
};

pub struct Clipboard {
    // could this just be a grid?
    clipboard: Vec<Vec<Option<CellType>>>,
    /// For calculating variable translation
    source_cell: (usize, usize),
    /// For tracking momentum direction
    last_paste_cell: (usize, usize),
    momentum: (i32, i32),
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            clipboard: Vec::new(),
            last_paste_cell: (0, 0),
            momentum: (0, 1),
            source_cell: (0, 0),
        }
    }

    /// Panics if clipboard is 0 length (if you call after you
    /// just filled it with anything you are gtg).
    pub fn qty(&self) -> usize {
        // it will be a square
        let x_len = self.clipboard.len();
        let y_len = self.clipboard[0].len();

        x_len * y_len
    }

    /// After pasting you gain momentum which can be used to
    /// to move the cursor in the same direction for the next
    /// paste.
    pub fn momentum(&self) -> (i32, i32) {
        let (x, y) = self.momentum;
        // prevent diagonal momentum
        if y != 0 { (0, y) } else { (x, 0) }
    }

    pub fn paste(&mut self, into: &mut Grid, translate: bool) {
        // cursor
        let (cx, cy) = into.cursor();

        // iterate thru the clipbaord's cells
        for (x, row) in self.clipboard.iter().enumerate() {
            for (y, cell) in row.iter().enumerate() {
                let idx = (x + cx, y + cy);

                if translate {
                    if let Some(cell) = cell {
                        let trans = cell.translate_cell(self.source_cell, into.cursor());
                        into.set_cell_raw(idx, Some(trans));
                    } else {
                        // The cell at this location doesn't exist (empty)
                        into.set_cell_raw::<CellType>(idx, None);
                    }
                } else {
                    // translate = false
                    into.set_cell_raw::<CellType>(idx, cell.clone());
                }
            }
        }

        let (lx, ly) = self.last_paste_cell;
        self.momentum = (cx as i32 - lx as i32, cy as i32 - ly as i32);
        self.last_paste_cell = (cx, cy);
    }

    /// Clones data from Grid into self.
    /// Start and end don't have to be sorted in any sort of way. The function works with
    /// any two points.
    pub fn clipboard_copy(&mut self, start: (usize, usize), end: (usize, usize), from: &Grid) {
        let (x1, y1) = start;
        let (x2, y2) = end;

        let (low_x, hi_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        let (low_y, hi_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };

        self.source_cell = (low_x, low_y);

        // size the clipboard appropriately
        self.clipboard.clear();
        // clone data into clipboard
        for x in low_x..=hi_x {
            let mut col = Vec::new();
            for y in low_y..=hi_y {
                let a = from.get_cell_raw(x, y);
                col.push(a.clone());
            }
            self.clipboard.push(col);
        }
        self.last_paste_cell = (low_x, low_y);
    }

    pub fn clipboard_cut(&mut self, start: (usize, usize), end: (usize, usize), from: &mut Grid) {
        let (x1, y1) = start;
        let (x2, y2) = end;

        let (low_x, hi_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        let (low_y, hi_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };

        // size the clipboard appropriately
        self.clipboard.clear();
        // clone data into clipboard
        for x in low_x..=hi_x {
            let mut col = Vec::new();
            for y in low_y..=hi_y {
                let a = from.get_cell_raw(x, y);
                col.push(a.clone());
                from.set_cell_raw::<CellType>((x, y), None);
            }
            self.clipboard.push(col);
        }
        self.last_paste_cell = (low_x, low_y);
    }
}

#[test]
fn copy_paste() {
    let mut app = App::new();

    app.grid.set_cell("A0", "hello".to_string());
    app.grid.mv_cursor_to(0, 0);

    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');
    // yy will have set mode back to normal at this point

    assert_eq!(app.clipboard.clipboard.len(), 1);
    assert!(app.clipboard.clipboard[0][0].as_ref().is_some_and(|c| c.to_string() == "hello"));

    app.grid.mv_cursor_to(1, 1);
    Mode::process_key(&mut app, 'p');

    let a = app.grid.get_cell("B1").as_ref().expect("Should've been set by paste");
    assert_eq!(a.to_string(), "hello");
}

#[test]
fn momentum_y_pos() {
    let mut app = App::new();

    app.grid.set_cell("A0", "hello".to_string());
    app.grid.mv_cursor_to(0, 0);

    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');
    // yy will have set mode back to normal at this point

    app.grid.mv_cursor_to(0, 1);
    Mode::process_key(&mut app, 'p');

    assert_eq!(app.clipboard.momentum(), (0, 1));
}

#[test]
fn momentum_y_neg() {
    let mut app = App::new();

    app.grid.set_cell("A1", "hello".to_string());
    app.grid.mv_cursor_to(0, 1);

    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');
    // yy will have set mode back to normal at this point

    app.grid.mv_cursor_to(0, 0);
    Mode::process_key(&mut app, 'p');

    assert_eq!(app.clipboard.momentum(), (0, -1));
}

#[test]
fn momentum_x_pos() {
    let mut app = App::new();

    app.grid.set_cell("A0", "hello".to_string());
    app.grid.mv_cursor_to(0, 0);

    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');
    // yy will have set mode back to normal at this point

    app.grid.mv_cursor_to(1, 0);
    Mode::process_key(&mut app, 'p');

    assert_eq!(app.clipboard.momentum(), (1, 0));
}

#[test]
fn momentum_x_neg() {
    let mut app = App::new();

    app.grid.set_cell("B0", "hello".to_string());
    app.grid.mv_cursor_to(1, 0);

    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');
    // yy will have set mode back to normal at this point

    app.grid.mv_cursor_to(0, 0);
    Mode::process_key(&mut app, 'p');

    assert_eq!(app.clipboard.momentum(), (-1, 0));
}

#[test]
fn diagonal_momentum() {
    let mut app = App::new();

    app.grid.set_cell("A1", "hello".to_string());
    app.grid.mv_cursor_to(0, 1);

    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');
    // yy will have set mode back to normal at this point

    app.grid.mv_cursor_to(1, 0);
    assert_eq!(app.grid.cursor(), (1, 0));
    Mode::process_key(&mut app, 'p');

    assert_eq!(app.clipboard.momentum(), (0, -1));
    assert_eq!(app.grid.cursor(), (1, 0));

    app.grid.apply_momentum(app.clipboard.momentum());

    assert_eq!(app.grid.cursor(), (1, 0));
}

#[test]
fn copy_paste_vars_translate() {
    let mut app = App::new();

    //  Translate Right ====================================================
    // A0 = A1 = 1
    app.grid.set_cell("A0", "=A1".to_string());
    app.grid.set_cell("A1", 1.);

    // Copy A0
    app.grid.mv_cursor_to(0, 0);
    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');

    assert!(app.clipboard.clipboard[0][0].as_ref().is_some_and(|c| c.to_string() == "=A1"));

    // Move cursor to B0
    app.grid.mv_cursor_to(1, 0);
    Mode::process_key(&mut app, 'p');

    let a = app.grid.get_cell("B0").as_ref().expect("Should've been set by paste");
    assert_eq!(a.to_string(), "=B1");

    // Translate Left ====================================================
    // Copy B0
    app.grid.mv_cursor_to(1, 0);
    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');

    // Move cursor to A0
    app.grid.mv_cursor_to(0, 0);
    Mode::process_key(&mut app, 'p');

    let a = app.grid.get_cell("A0").as_ref().expect("Should've been set by paste");
    assert_eq!(a.to_string(), "=A1");

    // Translate Down ====================================================
    // Copy A0
    app.grid.mv_cursor_to(0, 0);
    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');

    // Move cursor to A0
    app.grid.mv_cursor_to(0, 1);
    Mode::process_key(&mut app, 'p');

    let a = app.grid.get_cell("A1").as_ref().expect("Should've been set by paste");
    assert_eq!(a.to_string(), "=A2");

    // Translate Up ====================================================
    // Copy A1
    app.grid.mv_cursor_to(0, 1);
    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');

    // Move cursor to A0
    app.grid.mv_cursor_to(0, 0);
    Mode::process_key(&mut app, 'p');

    let a = app.grid.get_cell("A0").as_ref().expect("Should've been set by paste");
    assert_eq!(a.to_string(), "=A1");
}

#[test]
fn copy_paste_double_locked_var() {
    let mut app = App::new();

    app.grid.set_cell("A0", 0.);
    app.grid.set_cell("A1", "=$A$0".to_string());

    // Copy A0
    app.grid.mv_cursor_to(0, 1);
    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');

    app.grid.mv_cursor_to(1, 0);
    Mode::process_key(&mut app, 'p');
    let c = app.grid.get_cell("B0").as_ref().expect("Just set it");
    assert_eq!(c.to_string(), "=$A$0");
}

#[test]
fn copy_paste_x_locked_var() {
    let mut app = App::new();

    app.grid.set_cell("A0", 0.);
    app.grid.set_cell("A1", "=$A0".to_string());

    // Copy A0
    app.grid.mv_cursor_to(0, 1);
    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');

    app.grid.mv_cursor_to(1, 2);
    Mode::process_key(&mut app, 'p');
    let c = app.grid.get_cell("B2").as_ref().expect("Just set it");
    assert_eq!(c.to_string(), "=$A1");
}

#[test]
fn copy_paste_y_locked_var() {
    let mut app = App::new();

    app.grid.set_cell("A0", 0.);
    app.grid.set_cell("A1", "=A$0".to_string());

    // Copy A0
    app.grid.mv_cursor_to(0, 1);
    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');

    app.grid.mv_cursor_to(1, 2);
    Mode::process_key(&mut app, 'p');
    let c = app.grid.get_cell("B2").as_ref().expect("Just set it");
    assert_eq!(c.to_string(), "=B$0");
}

#[test]
fn copy_paste_var_in_function() {
    let mut app = App::new();
    app.grid.set_cell("A0", 4.to_string());
    Mode::process_key(&mut app, 'j');

    app.grid.set_cell("A1", "=math::log2(A0)".to_string());
    app.mode = super::mode::Mode::Chord(Chord::new('y'));
    Mode::process_key(&mut app, 'y');
    Mode::process_key(&mut app, 'j');
    Mode::process_key(&mut app, 'p');

    let a = app.grid.get_cell("A2").as_ref().expect("Should've been set by paste");
    assert_eq!(a.to_string(), "=math::log2(A1)");
}
