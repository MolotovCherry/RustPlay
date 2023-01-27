use egui::Vec2;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use egui::Id;

pub type TermOutput = Arc<Mutex<String>>;

#[derive(Debug, Default)]
pub struct Terminal {
    // the arc mutex string holds access to the terminal buffer
    // first is stdout, second is stderr
    pub content: HashMap<Id, (TermOutput, TermOutput)>,
    // the first Id is simply the tab id, the second is the abort ctx tmp Id
    //
    // this holds access to an abort process signal in ctx tmp memory
    // just remove the tmp ctx entry to drop it
    // the entry is type Arc<Mutex<Sender<()>>>
    pub abortable: HashMap<Id, Id>,
    pub open: bool,
    pub scroll_offset: HashMap<Id, Vec2>,
    pub active_tab: Option<Id>,
    pub opened_from_close: bool,
    pub opened_from_close_dragging: bool,
    pub closed_from_open: bool,
}
