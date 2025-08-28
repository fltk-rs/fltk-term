use fltk::enums::{Color, Font};
use fltk::text::StyleTableEntry;
use vte::Params;

const STYLE_MAP: [(char, Color, Font); 16] = [
    ('A', Color::from_hex(0x282828), Font::Courier),    // Black (ANSI 30)
    ('B', Color::Red, Font::Courier),                   // Red (ANSI 31)
    ('C', Color::Green, Font::Courier),                 // Green (ANSI 32)
    ('D', Color::Yellow, Font::Courier),                // Yellow (ANSI 33)
    ('E', Color::Blue, Font::Courier),                  // Blue (ANSI 34)
    ('F', Color::Magenta, Font::Courier),               // Magenta (ANSI 35)
    ('G', Color::Cyan, Font::Courier),                  // Cyan (ANSI 36)
    ('H', Color::White, Font::Courier),                 // White (ANSI 37)
    ('I', Color::from_hex(0x666666), Font::CourierBold), // Bright Black (ANSI 90)
    ('J', Color::from_hex(0xf14c4c), Font::CourierBold), // Bright Red (ANSI 91)
    ('K', Color::from_hex(0x23d186), Font::CourierBold), // Bright Green (ANSI 92)
    ('L', Color::from_hex(0xf5f543), Font::CourierBold), // Bright Yellow (ANSI 93)
    ('M', Color::from_hex(0x3b8eea), Font::CourierBold), // Bright Blue (ANSI 94)
    ('N', Color::from_hex(0xd670d6), Font::CourierBold), // Bright Magenta (ANSI 95)
    ('O', Color::from_hex(0x29b8db), Font::CourierBold), // Bright Cyan (ANSI 96)
    ('P', Color::White, Font::CourierBold),             // Bright White (ANSI 97)
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnsiState {
    pub fg: u32,
    pub bg: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub blink: bool,
    pub reverse: bool,
}

impl AnsiState {
    pub fn new() -> Self {
        Self {
            fg: 7, // Default to white (ANSI 37) instead of black (ANSI 30)
            bg: 0, // Default background
            ..Default::default()
        }
    }

    pub fn reset_attributes(&mut self) {
        *self = Self::new();
    }

    pub fn get_style_char(&self) -> char {
        let mut style_index = 0;
        if self.bold {
            style_index += 8;
        }
        let fg_index = match self.fg {
            0..=7 => self.fg as usize,
            _ => 0,
        };
        style_index += fg_index;
        STYLE_MAP[style_index].0
    }

    pub fn get_style_table() -> Vec<StyleTableEntry> {
        let mut table = Vec::new();
        for &(_, color, font) in &STYLE_MAP {
            table.push(StyleTableEntry {
                color,
                font,
                size: 14,
            });
        }
        table
    }
}

pub fn parse_sgr_params(params: &Params, ansi_state: &mut AnsiState) {
    if params.is_empty() {
        ansi_state.reset_attributes();
        return;
    }

    let mut iter = params.iter();
    while let Some(param) = iter.next() {
        let val = param[0];
        match val {
            0 => ansi_state.reset_attributes(),
            1 => ansi_state.bold = true,
            3 => ansi_state.italic = true,
            4 => ansi_state.underline = true,
            5 => ansi_state.blink = true,
            7 => ansi_state.reverse = true,
            9 => ansi_state.strikethrough = true,
            22 => ansi_state.bold = false,
            23 => ansi_state.italic = false,
            24 => ansi_state.underline = false,
            25 => ansi_state.blink = false,
            27 => ansi_state.reverse = false,
            29 => ansi_state.strikethrough = false,
            30..=37 => {
                ansi_state.fg = (val - 30) as u32;
            }
            38 => {
                if let Some(param2) = iter.next() {
                    match param2[0] {
                        5 => {
                            if let Some(param3) = iter.next() {
                                ansi_state.fg = param3[0] as u32;
                            }
                        }
                        2 => {
                            if let (Some(r), Some(g), Some(b)) = (iter.next(), iter.next(), iter.next()) {
                                ansi_state.fg =
                                    ((r[0] as u32) << 16) + ((g[0] as u32) << 8) + (b[0] as u32);
                            }
                        }
                        _ => {}
                    }
                }
            }
            39 => ansi_state.fg = 0,
            40..=47 => {
                ansi_state.bg = (val - 40) as u32;
            }
            48 => {
                if let Some(param2) = iter.next() {
                    match param2[0] {
                        5 => {
                            if let Some(param3) = iter.next() {
                                ansi_state.bg = param3[0] as u32;
                            }
                        }
                        2 => {
                            if let (Some(r), Some(g), Some(b)) = (iter.next(), iter.next(), iter.next()) {
                                ansi_state.bg =
                                    ((r[0] as u32) << 16) + ((g[0] as u32) << 8) + (b[0] as u32);
                            }
                        }
                        _ => {}
                    }
                }
            }
            49 => ansi_state.bg = 0,
            90..=97 => {
                ansi_state.fg = (val - 90 + 8) as u32;
            }
            100..=107 => {
                ansi_state.bg = (val - 100 + 8) as u32;
            }
            _ => {}
        }
    }
}
