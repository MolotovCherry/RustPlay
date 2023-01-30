use egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub ansi_colors: AnsiColors,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash)]
pub struct AnsiColors {
    pub black: Rgb,
    pub red: Rgb,
    pub green: Rgb,
    pub yellow: Rgb,
    pub blue: Rgb,
    pub magenta: Rgb,
    pub cyan: Rgb,
    pub white: Rgb,
    pub bright_black: Rgb,
    pub bright_red: Rgb,
    pub bright_green: Rgb,
    pub bright_yellow: Rgb,
    pub bright_blue: Rgb,
    pub bright_magenta: Rgb,
    pub bright_cyan: Rgb,
    pub bright_white: Rgb,
}

impl Default for AnsiColors {
    fn default() -> Self {
        Self {
            black: Rgb(12, 12, 12),
            red: Rgb(197, 15, 31),
            green: Rgb(19, 161, 14),
            yellow: Rgb(193, 156, 0),
            blue: Rgb(0, 55, 218),
            magenta: Rgb(136, 23, 152),
            cyan: Rgb(58, 150, 221),
            white: Rgb(204, 204, 204),
            bright_black: Rgb(118, 118, 118),
            bright_red: Rgb(231, 72, 86),
            bright_green: Rgb(22, 198, 12),
            bright_yellow: Rgb(249, 241, 165),
            bright_blue: Rgb(59, 120, 255),
            bright_magenta: Rgb(180, 0, 158),
            bright_cyan: Rgb(97, 214, 214),
            bright_white: Rgb(242, 242, 242),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn to_color32(self) -> Color32 {
        Color32::from_rgb(self.0, self.1, self.2)
    }
}
