use serde::{Deserialize, Serialize};

use super::dock::DockConfig;
use super::theme::ThemeConfig;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub dock: DockConfig,
    pub theme: ThemeConfig,
}
