use std::collections::HashMap;

use egui::Id;

#[derive(Debug, Default)]
pub struct Terminal {
    pub content: HashMap<Id, String>,
    pub open: bool,
    pub opened_from_close: bool,
    pub opened_from_close_dragging: bool,
    pub closed_from_open: bool,
}
