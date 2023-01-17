use serde::{Deserialize, Serialize};

use super::dock::DockConfig;
use super::theme::ThemeConfig;
use super::GitHub;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing, skip_deserializing)]
    pub dock: DockConfig,
    pub github: GitHub,
    pub theme: ThemeConfig,
}
