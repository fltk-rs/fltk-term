use fltk::{
    enums::{Color, Font},
    text::StyleTableEntry,
};

// Basic 16-color ANSI palette
const BLACK: Color = Color::from_hex(0x282828); // Dark grey, not pure black
const RED: Color = Color::from_hex(0xcd3131);
const GREEN: Color = Color::from_hex(0x0dbc79);
const YELLOW: Color = Color::from_hex(0xe5e510);
const BLUE: Color = Color::from_hex(0x2472c8);
const MAGENTA: Color = Color::from_hex(0xbc3fbc);
const CYAN: Color = Color::from_hex(0x11a8cd);
const WHITE: Color = Color::from_hex(0xe5e5e5);
const BRIGHT_BLACK: Color = Color::from_hex(0x666666);
const BRIGHT_RED: Color = Color::from_hex(0xf14c4c);
const BRIGHT_GREEN: Color = Color::from_hex(0x23d186);
const BRIGHT_YELLOW: Color = Color::from_hex(0xf5f543);
const BRIGHT_BLUE: Color = Color::from_hex(0x3b8eea);
const BRIGHT_MAGENTA: Color = Color::from_hex(0xd670d6);
const BRIGHT_CYAN: Color = Color::from_hex(0x29b8db);
const BRIGHT_WHITE: Color = Color::from_hex(0xe5e5e5);

pub(crate) fn init() -> Vec<StyleTableEntry> {
    vec![
        // Regular-weight fonts
        StyleTableEntry { // 0
            color: BLACK,
            font: Font::Courier,
            size: 14,
        },
        StyleTableEntry { // 1
            color: RED,
            font: Font::Courier,
            size: 14,
        },
        StyleTableEntry { // 2
            color: GREEN,
            font: Font::Courier,
            size: 14,
        },
        StyleTableEntry { // 3
            color: YELLOW,
            font: Font::Courier,
            size: 14,
        },
        StyleTableEntry { // 4
            color: BLUE,
            font: Font::Courier,
            size: 14,
        },
        StyleTableEntry { // 5
            color: MAGENTA,
            font: Font::Courier,
            size: 14,
        },
        StyleTableEntry { // 6
            color: CYAN,
            font: Font::Courier,
            size: 14,
        },
        StyleTableEntry { // 7
            color: WHITE,
            font: Font::Courier,
            size: 14,
        },
        // Bold fonts
        StyleTableEntry { // 8
            color: BRIGHT_BLACK,
            font: Font::CourierBold,
            size: 14,
        },
        StyleTableEntry { // 9
            color: BRIGHT_RED,
            font: Font::CourierBold,
            size: 14,
        },
        StyleTableEntry { // 10
            color: BRIGHT_GREEN,
            font: Font::CourierBold,
            size: 14,
        },
        StyleTableEntry { // 11
            color: BRIGHT_YELLOW,
            font: Font::CourierBold,
            size: 14,
        },
        StyleTableEntry { // 12
            color: BRIGHT_BLUE,
            font: Font::CourierBold,
            size: 14,
        },
        StyleTableEntry { // 13
            color: BRIGHT_MAGENTA,
            font: Font::CourierBold,
            size: 14,
        },
        StyleTableEntry { // 14
            color: BRIGHT_CYAN,
            font: Font::CourierBold,
            size: 14,
        },
        StyleTableEntry { // 15
            color: BRIGHT_WHITE,
            font: Font::CourierBold,
            size: 14,
        },
    ]
}
