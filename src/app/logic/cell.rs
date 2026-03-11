use std::fmt::Display;

use evalexpr::eval_with_context;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Paragraph, Widget},
};

use crate::app::logic::{context::ExtractionContext, grid::Grid};

#[derive(Clone)]
pub enum FormatRule<T>
where
    T: PartialEq + PartialOrd,
{
    GT(T, Style),
    LT(T, Style),
    EQ(T, Style),
}

impl<T> Widget for &FormatRule<T>
where
    T: Display + PartialEq + PartialOrd,
{
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let split = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);

        let lh = split[0];
        let rh = split[1];

        let white = Style::default().fg(ratatui::style::Color::White).bg(ratatui::style::Color::Black);

        match self {
            FormatRule::GT(v, style) => {
                Paragraph::new(format!("> {v}")).style(white).render(lh, buf);
                Paragraph::new("xyz").style(*style).render(rh, buf);
            }
            FormatRule::LT(v, style) => {
                Paragraph::new(format!("< {v}")).style(white).render(lh, buf);
                Paragraph::new("xyz").style(*style).render(rh, buf);
            }
            FormatRule::EQ(v, style) => {
                Paragraph::new(format!("= {v}")).style(white).render(lh, buf);
                Paragraph::new("xyz").style(*style).render(rh, buf);
            }
        };
    }
}

impl<T> FormatRule<T>
where
    T: PartialEq + PartialOrd + Clone,
{
    fn does_rule_apply(&self, t: T) -> bool {
        match self {
            FormatRule::GT(v, _style) => *v > t,
            FormatRule::LT(v, _style) => *v < t,
            FormatRule::EQ(v, _style) => *v == t,
        }
    }

    pub fn get_threashold(&self) -> T {
        match self {
            FormatRule::GT(v, _style) => v.clone(),
            FormatRule::LT(v, _style) => v.clone(),
            FormatRule::EQ(v, _style) => v.clone(),
        }
    }

    pub fn get_style_mut(&mut self) -> &mut Style {
        match self {
            FormatRule::GT(_, style) => style,
            FormatRule::LT(_, style) => style,
            FormatRule::EQ(_, style) => style,
        }
    }

    pub fn get_style(&self) -> Style {
        match self {
            FormatRule::GT(_, style) => style.clone(),
            FormatRule::LT(_, style) => style.clone(),
            FormatRule::EQ(_, style) => style.clone(),
        }
    }

    pub fn style_string(&self) -> (String, String) {
        let s = self.get_style();
        let fg = if let Some(f) = s.fg { f.to_string() } else { "None".to_string() };
        let bg = if let Some(b) = s.bg { b.to_string() } else { "None".to_string() };
        (fg, bg)
    }

    pub fn sign_char(&self) -> char {
        match self {
            FormatRule::GT(_, _) => '>',
            FormatRule::LT(_, _) => '<',
            FormatRule::EQ(_, _) => '=',
        }
    }
}

#[derive(Clone, Default)]
pub struct Formatting {
    pub rules: Vec<FormatRule<f64>>,
}

impl Display for Formatting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.rules.len();
        if len == 1 { write!(f, "{len} rule") } else { write!(f, "{len} rules") }
    }
}

impl Formatting {
    pub fn eval_for_style(&self, v: f64) -> Style {
        for r in &self.rules {
            if r.does_rule_apply(v) {
                return r.get_style()
            }
        }
        Style::default()
    }
}

#[derive(Clone)]
pub struct Cell {
    pub value: Option<CellType>,
    pub formatting: Option<Formatting>,
}

impl Default for Cell {
    fn default() -> Self {
        Self { value: None, formatting: None }
    }
}

impl Cell {
    pub fn format_string(&self) -> String {
        if let Some(v) = &self.formatting {
            return v.to_string();
        }
        String::new()
    }

    pub fn value_string(&self) -> String {
        if let Some(v) = &self.value {
            return v.to_string();
        }
        String::new()
    }

    pub fn escaped_csv_string(&self) -> String {
        if let Some(v) = &self.value {
            return v.escaped_csv_string();
        }
        String::new()
    }
    /// `replace_fn` takes the string, the old value, and then the new value.
    /// It can be thought of as `echo $1 | sed s/$2/$3/g`
    pub fn custom_translate_cell(
        &self,
        from: (usize, usize),
        to: (usize, usize),
        replace_fn: impl Fn(&str, &str, &str) -> String,
    ) -> Cell {
        if let Some(v) = &self.value {
            match &v {
                // don't translate non-equations
                CellType::Number(_) | CellType::String(_) => return self.clone(),
                CellType::Equation(eq) => {
                    // Populate the context
                    let ctx = ExtractionContext::new();
                    let _ = eval_with_context(&eq[1..], &ctx);

                    let mut equation = eq.clone();
                    // translate standard vars A0 -> A1
                    // extract all the variables
                    for old_var in ctx.dump_vars() {
                        let mut lock_x = false;
                        let mut lock_y = false;

                        if old_var.contains('$') {
                            let locations = old_var
                                .char_indices()
                                .filter(|(_, c)| *c == '$')
                                .map(|(i, _)| i)
                                .collect::<Vec<usize>>();
                            match locations.len() {
                                1 => {
                                    if locations[0] == 0 {
                                        // locking the X axis (A,B,C...)
                                        lock_x = true;
                                    } else {
                                        // inside the string somewhere, gonna assume this means to lock Y (1,2,3...)
                                        lock_y = true;
                                    }
                                }
                                2 => {
                                    // Ignore this variable all together, effectively lockng X & Y
                                    continue;
                                }
                                _ => {
                                    // There are 0 or >2 "$" in this string.
                                    //
                                    // Could probably optimize the code or something so you only go over the string
                                    // once, instead of contains() then getting the indexes of where it is.
                                    // You could then put your no-$ code here.
                                }
                            }
                        }

                        if let Some((src_x, src_y)) = Grid::parse_to_idx(&old_var) {
                            // Use i32s instead of usize in case of negative numbers
                            let (x1, y1) = from;
                            let x1 = x1 as i32;
                            let y1 = y1 as i32;
                            let (x2, y2) = to;
                            let x2 = x2 as i32;
                            let y2 = y2 as i32;

                            let dest_x = if lock_x { src_x } else { (src_x as i32 + (x2 - x1)) as usize };

                            let dest_y = if lock_y { src_y } else { (src_y as i32 + (y2 - y1)) as usize };

                            let alpha = Grid::num_to_char(dest_x);

                            // Persist the "$" locking
                            let new_var = if lock_x {
                                format!("${alpha}{dest_y}")
                            } else if lock_y {
                                format!("{alpha}${dest_y}")
                            } else {
                                format!("{alpha}{dest_y}")
                            };

                            // swap out vars
                            equation = replace_fn(&equation, &old_var, &new_var);
                            // rolling = rolling.replace(&old_var, &new_var);
                        } else {
                            // why you coping invalid stuff, nerd?
                            //
                            // could be copying a range
                            if let Some(parts) = Grid::range_as_indices(&old_var) {
                                // how far is the movement?
                                let dx = to.0 as i32 - from.0 as i32;

                                let xs = parts.0 as i32;
                                let xe = parts.1 as i32;

                                // apply movement
                                let mut new_range_start = xs + dx;
                                let mut new_range_end = xe + dx;

                                // bottom out at 0
                                if new_range_start < 0 {
                                    new_range_start = 0;
                                }
                                if new_range_end < 0 {
                                    new_range_end = 0;
                                }

                                // convert the index back into a letter and then submit it
                                let start = Grid::num_to_char(new_range_start as usize);
                                let end = Grid::num_to_char(new_range_end as usize);
                                equation = replace_fn(&equation, &old_var, &format!("{start}:{end}"));
                            }
                        }
                    }
                    return equation.into();
                }
            }
        } else {
            Cell::default()
        }
    }

    pub fn translate_cell(&self, from: (usize, usize), to: (usize, usize)) -> Cell {
        self.custom_translate_cell(from, to, |a, b, c| a.replace(b, c))
    }
}

impl From<f64> for Cell {
    fn from(value: f64) -> Self {
        let v = CellType::duck_type(value.to_string());
        Cell { value: Some(v), formatting: None }
    }
}

impl From<String> for Cell {
    fn from(value: String) -> Self {
        let v = CellType::duck_type(value.to_string());
        Cell { value: Some(v), formatting: None }
    }
}

#[derive(Debug, Clone)]
pub enum CellType {
    Number(f64),
    String(String),
    Equation(String),
}

impl Default for CellType {
    fn default() -> Self {
        Self::String(String::default())
    }
}

impl From<f64> for CellType {
    fn from(value: f64) -> Self {
        CellType::duck_type(value.to_string())
    }
}

impl From<String> for CellType {
    fn from(value: String) -> Self {
        CellType::duck_type(value)
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
        if display.contains(CSV_DELIMITER) { format!("\"{display}\"") } else { display }
    }

    fn duck_type(value: impl Into<String>) -> Self {
        let value = value.into();

        if let Ok(parse) = value.parse::<f64>() {
            Self::Number(parse)
        } else {
            if value.starts_with('=') { Self::Equation(value) } else { Self::String(value) }
        }
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

impl PartialEq for CellType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Equation(left), Self::Equation(right)) => left == right,
            _ => false,
        }
    }
}
