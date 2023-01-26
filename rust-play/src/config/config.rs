use serde::{Deserialize, Serialize};

use super::dock::DockConfig;
use super::theme::ThemeConfig;
use super::GitHub;
use super::Terminal;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub github: GitHub,
    pub theme: ThemeConfig,

    // Runtime config and data sharing/saving, not persisted
    #[serde(skip_serializing, skip_deserializing)]
    pub dock: DockConfig,
    #[serde(skip_serializing, skip_deserializing)]
    pub terminal: Terminal,
}
