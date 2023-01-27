use crate::widgets::dock::{Tree, TreeTabs};
use egui::Id;
use egui_dock::NodeIndex;

#[derive(Debug)]
pub struct DockConfig {
    pub tree: Tree,
    pub commands: Vec<Command>,
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
    Play(Id),
}
