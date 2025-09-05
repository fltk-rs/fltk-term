use crate::cells::{CellBuffer, Style};
use crate::styles::*;
use core::cmp::min;
use fltk::enums::Color;
use vte::{Params, Perform};

pub struct CellsPerformer<'a> {
    pub buf: &'a mut CellBuffer,
    cur_style: Style,
    fg_base: Option<u16>,
    bg_base: Option<u16>,
}

impl<'a> CellsPerformer<'a> {
    pub fn new(buf: &'a mut CellBuffer) -> Self {
        let cur_style = Style::new(buf.default_bg, buf.default_fg);
        Self {
            buf,
            cur_style,
            fg_base: None,
            bg_base: None,
        }
    }
}

fn fg_from_code(code: u16) -> Option<Color> {
    match code {
        30 => Some(BLACK),
        31 => Some(RED),
        32 => Some(GREEN),
        33 => Some(YELLOW),
        34 => Some(BLUE),
        35 => Some(MAGENTA),
        36 => Some(CYAN),
        37 => Some(WHITE),
        // Bright variants (90-97)
        90 => Some(BRIGHT_BLACK),
        91 => Some(BRIGHT_RED),
        92 => Some(BRIGHT_GREEN),
        93 => Some(BRIGHT_YELLOW),
        94 => Some(BRIGHT_BLUE),
        95 => Some(BRIGHT_MAGENTA),
        96 => Some(BRIGHT_CYAN),
        97 => Some(BRIGHT_WHITE),
        _ => None,
    }
}

fn bg_from_code(code: u16) -> Option<Color> {
    match code {
        40 => Some(BLACK),
        41 => Some(RED),
        42 => Some(GREEN),
        43 => Some(YELLOW),
        44 => Some(BLUE),
        45 => Some(MAGENTA),
        46 => Some(CYAN),
        47 => Some(WHITE),
        // Bright variants (100-107)
        100 => Some(BRIGHT_BLACK),
        101 => Some(BRIGHT_RED),
        102 => Some(BRIGHT_GREEN),
        103 => Some(BRIGHT_YELLOW),
        104 => Some(BRIGHT_BLUE),
        105 => Some(BRIGHT_MAGENTA),
        106 => Some(BRIGHT_CYAN),
        107 => Some(BRIGHT_WHITE),
        _ => None,
    }
}

fn xterm256_to_color(idx: u16) -> Color {
    match idx {
        0 => Color::from_rgb(0, 0, 0),
        1 => Color::from_rgb(205, 0, 0),
        2 => Color::from_rgb(0, 205, 0),
        3 => Color::from_rgb(205, 205, 0),
        4 => Color::from_rgb(0, 0, 238),
        5 => Color::from_rgb(205, 0, 205),
        6 => Color::from_rgb(0, 205, 205),
        7 => Color::from_rgb(229, 229, 229),
        8 => Color::from_rgb(127, 127, 127),
        9 => Color::from_rgb(255, 0, 0),
        10 => Color::from_rgb(0, 255, 0),
        11 => Color::from_rgb(255, 255, 0),
        12 => Color::from_rgb(92, 92, 255),
        13 => Color::from_rgb(255, 0, 255),
        14 => Color::from_rgb(0, 255, 255),
        15 => Color::from_rgb(255, 255, 255),
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i % 36) / 6;
            let b = i % 6;
            let conv = |v: u16| -> u8 { [0, 95, 135, 175, 215, 255][min(v as usize, 5)] as u8 };
            Color::from_rgb(conv(r), conv(g), conv(b))
        }
        232..=255 => {
            let shade = 8 + 10 * (idx - 232);
            let c = shade as u8;
            Color::from_rgb(c, c, c)
        }
        _ => WHITE,
    }
}

impl Perform for CellsPerformer<'_> {
    fn print(&mut self, c: char) {
        self.buf.write_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                // Line feed - move cursor down and to beginning of line
                self.buf.line_feed();
            }
            b'\r' => {
                // Carriage return - move cursor to beginning of current line
                self.buf.carriage_return();
            }
            b'\t' => {
                // Horizontal Tab - advance to next 8-col tab stop
                self.buf.tab();
            }
            8 => {
                // backspace
                let (row, col) = self.buf.cursor();
                if col > 0 {
                    self.buf.move_cursor_abs(row, col - 1);
                }
            }
            7 => {
                // Bell character - ignore for now (could add system beep later)
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, c: char) {
        match c {
            'm' => {
                // SGR
                if params.is_empty() {
                    self.cur_style = Style::new(self.buf.default_bg, self.buf.default_fg);
                    self.buf.set_style(self.cur_style);
                    return;
                }
                let mut it = params.iter().peekable();
                while let Some(p) = it.next() {
                    let n = p[0];
                    match n {
                        0 => {
                            self.cur_style = Style::new(self.buf.default_bg, self.buf.default_fg);
                            self.fg_base = None;
                            self.bg_base = None;
                        }
                        1 => {
                            self.cur_style.bold = true;
                        }
                        2 => {
                            self.cur_style.faint = true;
                        }
                        3 => {
                            self.cur_style.italic = true;
                        }
                        4 => {
                            self.cur_style.underline = true;
                        }
                        5 => {
                            // blink (visual noop in current renderer)
                        }
                        7 => {
                            self.cur_style.inverse = true;
                        }
                        8 => {
                            // conceal; treat as faint for now
                            self.cur_style.faint = true;
                        }
                        9 => {
                            self.cur_style.strikethrough = true;
                        }
                        21 => {
                            // double underline -> map to underline true
                            self.cur_style.underline = true;
                        }
                        22 => {
                            self.cur_style.bold = false;
                            self.cur_style.faint = false;
                        }
                        23 => {
                            self.cur_style.italic = false;
                        }
                        24 => {
                            self.cur_style.underline = false;
                        }
                        25 => {
                            // blink off
                        }
                        27 => {
                            self.cur_style.inverse = false;
                        }
                        28 => {
                            self.cur_style.faint = false;
                        }
                        29 => {
                            self.cur_style.strikethrough = false;
                        }
                        53 => {
                            self.cur_style.overline = true;
                        }
                        55 => {
                            self.cur_style.overline = false;
                        }
                        30..=37 => {
                            self.fg_base = Some(n);
                            if let Some(c) = fg_from_code(n) {
                                self.cur_style.fg = c;
                            }
                        }
                        90..=97 => {
                            self.fg_base = None;
                            if let Some(c) = fg_from_code(n) {
                                self.cur_style.fg = c;
                            }
                        }
                        39 => {
                            self.cur_style.fg = WHITE;
                            self.fg_base = None;
                        }
                        40..=47 => {
                            self.bg_base = Some(n);
                            if let Some(c) = bg_from_code(n) {
                                self.cur_style.bg = c;
                            }
                        }
                        100..=107 => {
                            self.bg_base = None;
                            if let Some(c) = bg_from_code(n) {
                                self.cur_style.bg = c;
                            }
                        }
                        49 => {
                            self.cur_style.bg = BLACK;
                            self.bg_base = None;
                        }
                        38 => {
                            if let Some(mode) = it.next() {
                                match mode[0] {
                                    5 => {
                                        if let Some(col) = it.next() {
                                            self.cur_style.fg = xterm256_to_color(col[0]);
                                            self.fg_base = None;
                                        }
                                    }
                                    2 => {
                                        let r = it.next().map(|x| x[0] as u8).unwrap_or(255);
                                        let g = it.next().map(|x| x[0] as u8).unwrap_or(255);
                                        let b = it.next().map(|x| x[0] as u8).unwrap_or(255);
                                        self.cur_style.fg = Color::from_rgb(r, g, b);
                                        self.fg_base = None;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        48 => {
                            if let Some(mode) = it.next() {
                                match mode[0] {
                                    5 => {
                                        if let Some(col) = it.next() {
                                            self.cur_style.bg = xterm256_to_color(col[0]);
                                            self.bg_base = None;
                                        }
                                    }
                                    2 => {
                                        let r = it.next().map(|x| x[0] as u8).unwrap_or(0);
                                        let g = it.next().map(|x| x[0] as u8).unwrap_or(0);
                                        let b = it.next().map(|x| x[0] as u8).unwrap_or(0);
                                        self.cur_style.bg = Color::from_rgb(r, g, b);
                                        self.bg_base = None;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                self.buf.set_style(self.cur_style);
            }
            'K' => {
                // EL - Erase in Line: 0 right, 1 left, 2 entire
                let mode = params.iter().next().map(|p| p[0]).unwrap_or(0);
                match mode {
                    0 => self.buf.clear_eol_0(),
                    1 => self.buf.clear_eol_1(),
                    2 => self.buf.clear_eol_2(),
                    _ => {}
                }
            }
            'H' | 'f' => {
                // CUP: row;col (1-based)
                let mut it = params.iter();
                let row = it
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                let col = it
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.buf.move_cursor_abs(row, col);
            }
            'G' => {
                // CHA: horizontal absolute column
                let col = params
                    .iter()
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                let (row, _) = self.buf.cursor();
                self.buf.move_cursor_abs(row, col);
            }
            'A' => {
                // CUU: move up
                let count = params.iter().next().map(|p| p[0] as isize).unwrap_or(1);
                self.buf.move_cursor_rel(-{ count }, 0);
            }
            'B' => {
                // CUD: move down
                let count = params.iter().next().map(|p| p[0] as isize).unwrap_or(1);
                self.buf.move_cursor_rel(count, 0);
            }
            'C' => {
                // CUF: move forward
                let count = params.iter().next().map(|p| p[0] as isize).unwrap_or(1);
                self.buf.move_cursor_rel(0, count);
            }
            'D' => {
                // CUB: move backward
                let count = params.iter().next().map(|p| p[0] as isize).unwrap_or(1);
                self.buf.move_cursor_rel(0, -{ count });
            }
            'J' => {
                // ED - erase display: 0 to end, 1 from start, 2 entire, 3 clear scrollback
                let mode = params.iter().next().map(|p| p[0]).unwrap_or(0);
                match mode {
                    0 => self.buf.clear_ed_0(),
                    1 => self.buf.clear_ed_1(),
                    2 => self.buf.clear_ed_2(),
                    3 => self.buf.clear_scrollback(),
                    _ => {}
                }
            }
            '@' => {
                // ICH - insert blanks
                let count = params.iter().next().map(|p| p[0] as usize).unwrap_or(1);
                self.buf.insert_blanks(count);
            }
            'P' => {
                // DCH - delete characters
                let count = params.iter().next().map(|p| p[0] as usize).unwrap_or(1);
                self.buf.delete_chars(count);
            }
            'X' => {
                // ECH - erase characters (replace with blanks)
                let count = params.iter().next().map(|p| p[0] as usize).unwrap_or(1);
                self.buf.erase_chars(count);
            }
            'L' => {
                // IL - insert lines
                let count = params.iter().next().map(|p| p[0] as usize).unwrap_or(1);
                self.buf.insert_lines(count);
            }
            'M' => {
                // DL - delete lines
                let count = params.iter().next().map(|p| p[0] as usize).unwrap_or(1);
                self.buf.delete_lines(count);
            }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}
