use fltk::enums::Color;

// Basic 16-color ANSI palette
pub const BLACK: Color = Color::from_hex(0x181818); // Dark grey, not pure black
pub const RED: Color = Color::from_hex(0xcd3131);
pub const GREEN: Color = Color::from_hex(0x0dbc79);
pub const YELLOW: Color = Color::from_hex(0xe5e510);
pub const BLUE: Color = Color::from_hex(0x2472c8);
pub const MAGENTA: Color = Color::from_hex(0xbc3fbc);
pub const CYAN: Color = Color::from_hex(0x11a8cd);
pub const WHITE: Color = Color::from_hex(0xe5e5e5);
pub const BRIGHT_BLACK: Color = Color::from_hex(0x666666);
pub const BRIGHT_RED: Color = Color::from_hex(0xf14c4c);
pub const BRIGHT_GREEN: Color = Color::from_hex(0x23d186);
pub const BRIGHT_YELLOW: Color = Color::from_hex(0xf5f543);
pub const BRIGHT_BLUE: Color = Color::from_hex(0x3b8eea);
pub const BRIGHT_MAGENTA: Color = Color::from_hex(0xd670d6);
pub const BRIGHT_CYAN: Color = Color::from_hex(0x29b8db);
pub const BRIGHT_WHITE: Color = Color::from_hex(0xe5e5e5);