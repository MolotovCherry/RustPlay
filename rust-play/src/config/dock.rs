use crate::widgets::dock::{Tree, TreeTabs};
use egui::Id;
use egui_dock::NodeIndex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DockConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub tree: Tree,
    #[serde(skip_serializing, skip_deserializing)]
    pub commands: Vec<Command>,
    #[serde(skip_serializing, skip_deserializing)]
    pub counter: u32,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            tree: Tree::init(),
            commands: Default::default(),
            counter: 0,
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
    Rename(Id),
    Save(Id),
    Share(Id),
}

#[derive(Debug, Clone)]
pub enum TabCommand {
    Add(NodeIndex),
    Close(Id),
}
