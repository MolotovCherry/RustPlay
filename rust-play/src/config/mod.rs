#[allow(clippy::module_inception)]
mod config;
mod dock;
mod github;
mod terminal;
mod theme;

pub use config::*;
pub use dock::*;
pub use github::*;
pub use terminal::*;
pub use theme::*;
