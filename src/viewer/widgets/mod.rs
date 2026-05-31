use ratatui::widgets::{
    Widget,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CursorBox(pub u16, pub u16);

impl Widget for CursorBox {
    fn render(self, _area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized {
        let pos = (self.0, self.1);
        if let Some(cell) = buf.cell_mut(pos) {
            cell.set_char('#');
        }
    }
}
