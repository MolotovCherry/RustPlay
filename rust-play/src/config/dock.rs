use crate::utils::data::Data;
use crate::widgets::dock::{Tree, TreeTabs};
use egui::Rect;
use egui_dock::{NodeIndex, TabIndex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DockConfig {
    pub tree: Tree,
    #[serde(skip_serializing, skip_deserializing)]
    pub command: Vec<Command>,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            tree: Tree::init(),
            command: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    MenuCommand(MenuCommand),
    TabCommand(TabCommand),
}

#[derive(Debug, Clone)]
pub enum MenuCommand {
    Rename((NodeIndex, TabIndex, Rect)),
    Save((NodeIndex, TabIndex)),
    Share((NodeIndex, TabIndex)),
}

#[derive(Debug, Clone)]
pub enum TabCommand {
    Add(NodeIndex),
}
