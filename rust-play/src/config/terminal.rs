use egui::Vec2;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use egui::Id;

#[derive(Debug, Default)]
pub struct Terminal {
    pub content: HashMap<Id, String>,
    // receiver is reponsible for streaming the data from the subprocess to the terminal
    // The second ID is stored in ctx, which is used to terminate running processes
    pub streamable: HashMap<Id, (Receiver<String>, Id)>,
    pub open: bool,
    pub scroll_offset: HashMap<Id, Vec2>,
    pub active_tab: Option<Id>,
    pub opened_from_close: bool,
    pub opened_from_close_dragging: bool,
    pub closed_from_open: bool,
}
