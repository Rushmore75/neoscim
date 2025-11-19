use ratatui::{layout::{Constraint, Direction, Layout}, widgets::{Paragraph, Widget}};

pub struct Plot {
    x: usize,
    y: Vec<usize>,
}
impl Plot {
    pub fn new(x: usize, y: usize) -> Self {
        Self {
            x,
            y: vec![y],
        }
    }

    pub fn add_column(&mut self) {
        self.y.push(1)
    }
    pub fn del_column(&mut self) {
        self.y.pop();
    }

    pub fn process_key(&mut self, c: char) {

    }
}

impl Widget for &Plot {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        // plus 1 for x
        let columns = self.y.len() + 1;

        let mut constraints = Vec::new();
        for _ in 0..=columns {
            constraints.push(Constraint::Min(1));
        }

        Paragraph::new("Foobar").render(area, buf);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let x_space = layout[0];
    }
}