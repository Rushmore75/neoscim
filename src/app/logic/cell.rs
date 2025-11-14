use std::fmt::Display;

use evalexpr::eval_with_context;

use crate::app::logic::{calc::Grid, ctx::ExtractionContext};

#[derive(Debug, Clone)]
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

pub const CSV_DELIMITER: char = ',';
const CSV_ESCAPE: char = '"';

impl CellType {
    pub fn escaped_csv_string(&self) -> String {
        let mut display = self.to_string();

        // escape quotes " -> ""
        let needs_escaping = display.char_indices().filter(|f| f.1 == CSV_ESCAPE).map(|f| f.0).collect::<Vec<usize>>();
        for idx in needs_escaping.iter().rev() {
            display.insert(*idx, CSV_ESCAPE);
        }

        // escape string of it has a comma
        if display.contains(CSV_DELIMITER) {
            format!("\"{display}\"{CSV_DELIMITER}")
        } else {
            format!("{display}{CSV_DELIMITER}")
        }
    }

    fn duck_type<'a>(value: impl Into<String>) -> Self {
        let value = value.into();

        if let Ok(parse) = value.parse::<f64>() {
            Self::Number(parse)
        } else {
            if value.starts_with('=') { Self::Equation(value) } else { Self::String(value) }
        }
    }

    pub fn custom_translate_cell(&self, from: (usize, usize), to: (usize, usize), replace_fn: impl Fn(&str, &str, &str) -> String) -> CellType {
        match self {
            // don't translate non-equations
            CellType::Number(_) | CellType::String(_) => return self.clone(),
            CellType::Equation(eq) => {
                // extract all the variables
                let ctx = ExtractionContext::new();
                let _ = eval_with_context(eq, &ctx);

                let mut rolling = eq.clone();
                // translate standard vars A0 -> A1
                for old_var in ctx.dump_vars() {
                    let mut lock_x = false;
                    let mut lock_y = false;

                    if old_var.contains('$') {
                        let locations = old_var.char_indices().filter(|(_, c)| *c == '$').map(|(i, _)| i).collect::<Vec<usize>>();
                        match locations.len() {
                            1 => {
                                if locations[0] == 0 {
                                    // locking the X axis (A,B,C...)
                                    lock_x = true;
                                } else if locations[0] < old_var.len() {
                                    // inside the string somewhere, gonna assume this means to lock Y (1,2,3...)
                                    lock_y = true;

                                } else {
                                    // where tf is this dollar sign?
                                    // (It's somewhere malformed, like A0$ or something)
                                }
                            }
                            2 => {
                                // YOLO, lock both X & Y
                                continue; // just pretend you never even saw this var
                            }
                            _ => {
                                // Could probably optimize the code or something so you only go over the string
                                // once, instead of contains() then getting the indexes of where it is.
                                // You could then put your no-$ code here.
                            }
                        }
                    }
                    
                    if let Some((src_x, src_y)) = Grid::parse_to_idx(&old_var) {
                        let (x1, y1) = from;
                        let x1 = x1 as i32;
                        let y1 = y1 as i32;
                        let (x2, y2) = to;
                        let x2 = x2 as i32;
                        let y2 = y2 as i32;

                        let dest_x = if lock_x {
                            src_x as usize
                        } else {
                            (src_x as i32 + (x2 - x1)) as usize
                        };

                        let dest_y = if lock_y {
                            src_y as usize
                        } else {
                            (src_y as i32 + (y2 - y1)) as usize
                        };

                        let alpha = Grid::num_to_char(dest_x);
                        let alpha = alpha.trim();

                        let new_var = if lock_x {
                            format!("${alpha}{dest_y}")
                        } else if lock_y {
                            format!("{alpha}${dest_y}")
                        } else {
                            format!("{alpha}{dest_y}")
                        };


                        // swap out vars
                        rolling = replace_fn(&rolling, &old_var, &new_var);
                        // rolling = rolling.replace(&old_var, &new_var);
                    } else {
                        // why you coping invalid stuff, nerd?
                    }
                }
                return rolling.into();
            }
        }
    }

    pub fn translate_cell(&self, from: (usize, usize), to: (usize, usize)) -> CellType {
        self.custom_translate_cell(from, to, |a,b,c| a.replace(b, c))
    }
}

impl Display for CellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = match self {
            CellType::Number(n) => n.to_string(),
            CellType::String(n) => n.to_owned(),
            CellType::Equation(r) => r.to_owned(),
        };
        write!(f, "{d}")
    }
}
