use ansi_parser::AnsiSequence;
use ansi_parser::{AnsiParser as ParseAnsi, Output};

// parse color mode 5
fn parse_rgb(color: u8) -> Color {
    // 0-15 are regular colors, even in color mode 5
    if color < 16 {
        return match color {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::White,
            8 => Color::BrightBlack,
            9 => Color::BrightRed,
            10 => Color::BrightGreen,
            11 => Color::BrightYellow,
            12 => Color::BrightBlue,
            13 => Color::BrightMagenta,
            14 => Color::BrightCyan,
            15 => Color::BrightWhite,
            _ => unreachable!(),
        };
    }

    // extended range of colors
    if let 16..=231 = color {
        let index_r = (color - 16) / 36;
        let rgb_r = if index_r > 0 { 55 + index_r * 40 } else { 0 };
        let index_g = ((color - 16) % 36) / 6;
        let rgb_g = if index_g > 0 { 55 + index_g * 40 } else { 0 };
        let index_b = (color - 16) % 6;
        let rgb_b = if index_b > 0 { 55 + index_b * 40 } else { 0 };

        Color::Rgb(rgb_r, rgb_g, rgb_b)
    } else {
        // grayscale colors from 232-256
        let color = (color - 232) * 10 + 8;
        Color::Rgb(color, color, color)
    }
}

pub fn parse(text: &str) -> Parsed {
    let parsed = text.ansi_parse();

    let mut properties = vec![];

    // represent text style state
    let mut bold = false;
    let mut dim = false;
    let mut italic = false;
    let mut underline = false;
    let mut blink = false;
    let mut reverse = false;
    let mut hidden = false;
    let mut strikethrough = false;

    let mut fg = None;
    let mut bg = None;

    let mut text_counter = 0;

    for chunk in parsed {
        match chunk {
            Output::TextBlock(t) => {
                if t.is_empty() {
                    continue;
                }

                let style = TextStyle {
                    bold,
                    dim,
                    italic,
                    underline,
                    blink,
                    reverse,
                    hidden,
                    strikethrough,
                };

                let len = t.len();

                let property = TextProperty {
                    start: text_counter,
                    end: text_counter + len,
                    style,
                    fg,
                    bg,
                };

                if property.end > 0 {
                    properties.push(property);
                }

                text_counter += len;
            }

            Output::Escape(e) => {
                match e {
                    AnsiSequence::SetGraphicsMode(m) => {
                        // parse multi color codes independently
                        match m[0] {
                            38 => {
                                if m[1] == 5 {
                                    fg = Some(parse_rgb(m[2]));
                                } else if m[1] == 2 {
                                    fg = Some(Color::Rgb(m[2], m[3], m[4]));
                                }
                            }
                            48 => {
                                if m[1] == 5 {
                                    bg = Some(parse_rgb(m[2]));
                                } else if m[1] == 2 {
                                    bg = Some(Color::Rgb(m[2], m[3], m[4]));
                                }
                            }

                            _ => (),
                        }

                        // these can have multiple commands, so loop them
                        for c in m {
                            match c {
                                // reset all modes
                                0 => {
                                    bold = false;
                                    dim = false;
                                    italic = false;
                                    underline = false;
                                    blink = false;
                                    reverse = false;
                                    hidden = false;
                                    strikethrough = false;
                                    fg = None;
                                    bg = None;
                                }

                                // set bold -> 22 reset
                                1 => bold = true,

                                // set dim/faint -> 22 reset
                                2 => dim = true,

                                // set italic -> 23 reset
                                3 => italic = true,

                                // set underline -> 24 reset
                                4 => underline = true,

                                // set blink -> 25 reset
                                5 => blink = true,

                                // set inverse/reverse -> 27 reset
                                7 => reverse = true,

                                // set hidden -> 28 reset
                                8 => hidden = true,

                                // set strikethrough -> 29 reset
                                9 => strikethrough = true,

                                30 => fg = Some(Color::Black),
                                40 => bg = Some(Color::Black),

                                31 => fg = Some(Color::Red),
                                41 => bg = Some(Color::Red),

                                32 => fg = Some(Color::Green),
                                42 => bg = Some(Color::Green),

                                33 => fg = Some(Color::Yellow),
                                43 => bg = Some(Color::Yellow),

                                34 => fg = Some(Color::Blue),
                                44 => bg = Some(Color::Blue),

                                35 => fg = Some(Color::Magenta),
                                45 => bg = Some(Color::Magenta),

                                36 => fg = Some(Color::Cyan),
                                46 => bg = Some(Color::Cyan),

                                37 => fg = Some(Color::White),
                                47 => bg = Some(Color::White),

                                // Default
                                39 => fg = None,
                                49 => bg = None,

                                90 => fg = Some(Color::BrightBlack),
                                100 => bg = Some(Color::BrightBlack),

                                91 => fg = Some(Color::BrightRed),
                                101 => bg = Some(Color::BrightRed),

                                92 => fg = Some(Color::BrightGreen),
                                102 => bg = Some(Color::BrightGreen),

                                93 => fg = Some(Color::BrightYellow),
                                103 => bg = Some(Color::BrightYellow),

                                94 => fg = Some(Color::BrightBlue),
                                104 => bg = Some(Color::BrightBlue),

                                95 => fg = Some(Color::BrightMagenta),
                                105 => bg = Some(Color::BrightMagenta),

                                96 => fg = Some(Color::BrightCyan),
                                106 => bg = Some(Color::BrightCyan),

                                97 => fg = Some(Color::BrightWhite),
                                107 => bg = Some(Color::BrightWhite),

                                _ => break,
                            }
                        }
                    }

                    AnsiSequence::SetMode(_) => todo!(),
                    AnsiSequence::ResetMode(_) => todo!(),
                    _ => (),
                }
            }
        }
    }

    Parsed { properties }
}

#[derive(Debug)]
pub struct Parsed {
    pub properties: Vec<TextProperty>,
}

#[derive(Debug, Hash, Copy, Clone)]
pub struct TextProperty {
    pub start: usize,
    pub end: usize,
    pub style: TextStyle,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

#[derive(Debug, Copy, Clone, Default, Hash)]
pub struct TextStyle {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
}

#[derive(Debug, Copy, Clone, Hash)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Rgb(u8, u8, u8),
}
