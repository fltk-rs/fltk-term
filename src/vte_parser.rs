use crate::ansi;
use fltk::prelude::*;
use vte::{Params, Perform};

macro_rules! debug {
    ($($e:expr),+) => {
        {
            #[cfg(feature="debug-term")]
            {
                eprintln!($($e),+)
            }
            #[cfg(not(feature="debug-term"))]
            {
                ($($e),+)
            }
        }
    };
}

impl super::VteParser {
    fn set_cursor_position(&mut self, row: i32, col: i32) {
        if let Some(buf) = self.st.buffer() {
            let text = buf.text();
            let lines: Vec<&str> = text.split('\n').collect();

    
            let target_row = (row - 1).max(0) as usize;
            let target_col = (col - 1).max(0) as usize;

    
            let mut pos = 0i32;
            for (line_idx, line) in lines.iter().enumerate() {
                if line_idx == target_row {
            
            
                    pos += target_col.min(line.len() + 1) as i32;
                    break;
                } else if line_idx < target_row {
                    pos += line.len() as i32 + 1
                } else {
                    break;
                }
            }

            self.st.set_insert_position(pos);
        }
    }

    fn move_cursor_relative(&mut self, rows: i32, cols: i32) {
        if let Some(buf) = self.st.buffer() {
            let current_pos = self.st.insert_position();
            let text = buf.text();
            let lines: Vec<&str> = text.split('\n').collect();

    
            let mut current_row = 0usize;
            let mut current_col = 0usize;
            let mut char_count = 0i32;

            for (line_idx, line) in lines.iter().enumerate() {
                if char_count + line.len() as i32 >= current_pos {
                    current_row = line_idx;
                    current_col = (current_pos - char_count) as usize;
                    break;
                }
                char_count += line.len() as i32 + 1
            }

    
            let new_row = (current_row as i32 + rows).max(0) as usize;
            let new_col = (current_col as i32 + cols).max(0) as usize;

            self.set_cursor_position((new_row + 1) as i32, (new_col + 1) as i32);
        }
    }
}

impl Perform for super::VteParser {
    fn print(&mut self, c: char) {
        debug!("print: '{}'", c);
        self.temp_s.push(c);
        let style_char = self.ansi_state.get_style_char();
        self.temp_b.push(style_char);
    }

    fn execute(&mut self, byte: u8) {
        debug!("{}", byte);
        match byte {
            8 => {
        
                if let Some(mut buf) = self.st.buffer() {
                    let cursor_pos = self.st.insert_position();
                    if cursor_pos > 0 {
                
                        buf.remove(cursor_pos - 1, cursor_pos);
                        
                
                        if let Some(mut style_buf) = self.st.style_buffer() {
                            style_buf.remove(cursor_pos - 1, cursor_pos);
                        }
                        
                
                        self.st.set_insert_position(cursor_pos - 1);
                    }
                }
            }
            10 => {
        
                self.temp_s.push(byte as char);
                self.temp_b.push(self.ch);
            }
            13 => {
        
        
        
            }
            9 => {
        
                let current_col = self.temp_s.len() % 80
                let spaces_needed = 8 - (current_col % 8);
                for _ in 0..spaces_needed {
                    self.temp_s.push(' ');
                    self.temp_b.push(self.ch);
                }
            }
            0 | 7 => ()
            _ => (),
        }
    }

    fn hook(&mut self, params: &Params, intermediates: &[
        u8
    ], ignore: bool, c: char) {
        debug!(
            "[hook] params={:?}, intermediates={:?}, ignore={:?}, char={:?}",
            params, intermediates, ignore, c
        );
    }

    fn put(&mut self, byte: u8) {
        debug!("[put] {:02x}", byte);
    }

    fn unhook(&mut self) {
        debug!("[unhook]");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        debug!(
            "[osc_dispatch] params={:?} bell_terminated={}",
            params, bell_terminated
        );
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        debug!(
            "[csi_dispatch] params={:#?} intermediates={:?}, ignore={:?}, char={}",
            params, intermediates, ignore, c
        );
        match c {
            'm' => {
        
                ansi::parse_sgr_params(params, &mut self.ansi_state);
        
                self.ch = self.ansi_state.get_style_char();
            }
            'K' => {
        
                let param = if params.is_empty() {
                    0
                } else {
                    params.iter().next().unwrap()[0]
                };
                match param {
                    0 => {
                
                        if let Some(mut buf) = self.st.buffer() {
                            let text = buf.text();
                            let pos = self.st.insert_position();

                    
                            let mut end_pos = pos;
                            let chars: Vec<char> = text.chars().collect();
                            while end_pos < chars.len() as i32 && chars[end_pos as usize] != '\n' {
                                end_pos += 1;
                            }

                            if pos < end_pos {
                                buf.remove(pos, end_pos);
                                if let Some(mut style_buf) = self.st.style_buffer() {
                                    style_buf.remove(pos, end_pos);
                                }
                            }
                        }
                    }
                    1 => {
                
                        if let Some(mut buf) = self.st.buffer() {
                            let text = buf.text();
                            let pos = self.st.insert_position();

                    
                            let mut start_pos = pos;
                            let chars: Vec<char> = text.chars().collect();
                            while start_pos > 0 && chars[(start_pos - 1) as usize] != '\n' {
                                start_pos -= 1;
                            }

                            if start_pos < pos {
                                buf.remove(start_pos, pos);
                                if let Some(mut style_buf) = self.st.style_buffer() {
                                    style_buf.remove(start_pos, pos);
                                }
                            }
                        }
                    }
                    2 => {
                
                        if let Some(mut buf) = self.st.buffer() {
                            let text = buf.text();
                            let pos = self.st.insert_position();

                    
                            let chars: Vec<char> = text.chars().collect();
                            let mut start_pos = pos;
                            let mut end_pos = pos;

                            while start_pos > 0 && chars[(start_pos - 1) as usize] != '\n' {
                                start_pos -= 1;
                            }

                            while end_pos < chars.len() as i32 && chars[end_pos as usize] != '\n' {
                                end_pos += 1;
                            }

                            if start_pos < end_pos {
                                buf.remove(start_pos, end_pos);
                                if let Some(mut style_buf) = self.st.style_buffer() {
                                    style_buf.remove(start_pos, end_pos);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            'A' => {
        
                let count = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                self.move_cursor_relative(-count, 0);
            }
            'B' => {
        
                let count = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                self.move_cursor_relative(count, 0);
            }
            'C' => {
        
                debug!("Cursor right - no action needed");
            }
            'D' => {
        
                debug!("Cursor left - no action needed");
            }
            'H' | 'f' => {
        
                if params.len() > 1 {
                    let mut iter = params.iter();
                    if let (Some(row_param), Some(col_param)) = (iter.next(), iter.next()) {
                        let row = row_param[0] as i32;
                        let col = col_param[0] as i32;
                        self.set_cursor_position(row, col);
                    }
                } else {
            
                    self.set_cursor_position(1, 1);
                }
            }
            'G' => {
        
                let col = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                if let Some(buf) = self.st.buffer() {
                    let current_pos = self.st.insert_position();
                    let text = buf.text();
                    let lines: Vec<&str> = text.split('\n').collect();

            
                    let mut current_row = 0usize;
                    let mut char_count = 0i32;

                    for (line_idx, line) in lines.iter().enumerate() {
                        if char_count + line.len() as i32 >= current_pos {
                            current_row = line_idx;
                            break;
                        }
                        char_count += line.len() as i32 + 1
                    }

                    self.set_cursor_position((current_row + 1) as i32, col);
                }
            }
            'd' => {
        
                let row = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                if let Some(buf) = self.st.buffer() {
                    let current_pos = self.st.insert_position();
                    let text = buf.text();
                    let lines: Vec<&str> = text.split('\n').collect();

            
                    let mut current_col = 0usize;
                    let mut char_count = 0i32;

                    for (_line_idx, line) in lines.iter().enumerate() {
                        if char_count + line.len() as i32 >= current_pos {
                            current_col = (current_pos - char_count) as usize;
                            break;
                        }
                        char_count += line.len() as i32 + 1
                    }

                    self.set_cursor_position(row, (current_col + 1) as i32);
                }
            }
            's' => {
        
                let pos = self.st.insert_position();
                if let Some(buf) = self.st.buffer() {
                    let text_before_cursor = &buf.text()[0..pos as usize];
                    let lines: Vec<&str> = text_before_cursor.split('\n').collect();
                    let row = lines.len() as i32;
                    let col = lines.last().map(|line| line.len() as i32).unwrap_or(0);
                    self.saved_cursor_pos = Some((row, col));
                }
            }
            'u' => {
        
                if let Some((row, col)) = self.saved_cursor_pos {
                    self.set_cursor_position(row, col);
                }
            }
            'J' => {
        
                let param = if params.is_empty() {
                    0
                } else {
                    params.iter().next().unwrap()[0]
                };
                match param {
                    0 => {
                
                        if let Some(mut buf) = self.st.buffer() {
                            let pos = self.st.insert_position();
                            let len = buf.length();
                            if pos < len {
                                buf.remove(pos, len);
                                if let Some(mut style_buf) = self.st.style_buffer() {
                                    style_buf.remove(pos, len);
                                }
                            }
                        }
                    }
                    1 => {
                
                        if let Some(mut buf) = self.st.buffer() {
                            let pos = self.st.insert_position();
                            if pos > 0 {
                                buf.remove(0, pos);
                                if let Some(mut style_buf) = self.st.style_buffer() {
                                    style_buf.remove(0, pos);
                                }
                            }
                        }
                    }
                    2 => {
                
                        if let Some(mut buf) = self.st.buffer() {
                            buf.set_text("");
                        }
                        if let Some(mut style_buf) = self.st.style_buffer() {
                            style_buf.set_text("");
                        }
                    }
                    3 => {
                
                        if let Some(mut buf) = self.st.buffer() {
                            buf.set_text("");
                        }
                        if let Some(mut style_buf) = self.st.style_buffer() {
                            style_buf.set_text("");
                        }
                
                        if let Ok(mut scrollback) = self.scrollback.lock() {
                            scrollback.clear();
                        }
                    }
                    _ => {}
                }
            }
            'L' => {
        
                let count = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                if let Some(mut buf) = self.st.buffer() {
                    let current_pos = self.st.insert_position();
                    let newlines = "\n".repeat(count as usize);
                    buf.insert(current_pos, &newlines);
                    if let Some(mut style_buf) = self.st.style_buffer() {
                        let style_chars = self.ch.to_string().repeat(count as usize);
                        style_buf.insert(current_pos, &style_chars);
                    }
                }
            }
            'M' => {
        
                let count = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                if let Some(mut buf) = self.st.buffer() {
                    let text = buf.text();
                    let current_pos = self.st.insert_position();
                    let lines: Vec<&str> = text.split('\n').collect();

            
                    let mut char_count = 0i32;
                    let mut start_line_idx = 0;
                    for (line_idx, line) in lines.iter().enumerate() {
                        if char_count + line.len() as i32 >= current_pos {
                            start_line_idx = line_idx;
                            break;
                        }
                        char_count += line.len() as i32 + 1;
                    }

            
                    let mut delete_start = 0i32;
                    let mut delete_end = 0i32;
                    char_count = 0;

                    for (line_idx, line) in lines.iter().enumerate() {
                        if line_idx == start_line_idx {
                            delete_start = char_count;
                        }
                        if line_idx == start_line_idx + (count as usize) {
                            delete_end = char_count;
                            break;
                        }
                        if line_idx >= start_line_idx && line_idx < start_line_idx + (count as usize) {
                            delete_end = char_count + line.len() as i32 + 1;
                        }
                        char_count += line.len() as i32 + 1;
                    }

                    if delete_start < delete_end {
                        buf.remove(delete_start, delete_end);
                        if let Some(mut style_buf) = self.st.style_buffer() {
                            style_buf.remove(delete_start, delete_end);
                        }
                    }
                }
            }
            'P' => {
        
                let count = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                if let Some(mut buf) = self.st.buffer() {
                    let current_pos = self.st.insert_position();
                    let end_pos = (current_pos + count).min(buf.length());
                    if current_pos < end_pos {
                        buf.remove(current_pos, end_pos);
                        if let Some(mut style_buf) = self.st.style_buffer() {
                            style_buf.remove(current_pos, end_pos);
                        }
                    }
                }
            }
            'X' => {
        
                let count = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                if let Some(mut buf) = self.st.buffer() {
                    let current_pos = self.st.insert_position();
                    let end_pos = (current_pos + count).min(buf.length());
                    if current_pos < end_pos {
                        let spaces = " ".repeat((end_pos - current_pos) as usize);
                        buf.remove(current_pos, end_pos);
                        buf.insert(current_pos, &spaces);
                        if let Some(mut style_buf) = self.st.style_buffer() {
                            style_buf.remove(current_pos, end_pos);
                            let style_chars =
                                self.ch.to_string().repeat((end_pos - current_pos) as usize);
                            style_buf.insert(current_pos, &style_chars);
                        }
                    }
                }
            }
            '@' => {
        
                let count = if let Some(p) = params.iter().next() {
                    p[0] as i32
                } else {
                    1
                };
                if let Some(mut buf) = self.st.buffer() {
                    let current_pos = self.st.insert_position();
                    let spaces = " ".repeat(count as usize);
                    buf.insert(current_pos, &spaces);
                    if let Some(mut style_buf) = self.st.style_buffer() {
                        let style_chars = self.ch.to_string().repeat(count as usize);
                        style_buf.insert(current_pos, &style_chars);
                    }
                }
            }
            'h' => {
        
                if let Some(param) = params.iter().next() {
                    match param[0] {
                        1047 | 47 => {
                    
                    
                            debug!("Switch to alternate screen buffer");
                        }
                        25 => {
                    
                            debug!("Show cursor");
                        }
                        _ => {}
                    }
                }
            }
            'l' => {
        
                if let Some(param) = params.iter().next() {
                    match param[0] {
                        1047 | 47 => {
                    
                            debug!("Switch to main screen buffer");
                        }
                        25 => {
                    
                            debug!("Hide cursor");
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        debug!(
            "[esc_dispatch] intermediates={:?}, ignore={:?}, byte={:02x}",
            intermediates, ignore, byte
        );

        match byte {
    
            55 => 
                let pos = self.st.insert_position();
        
                if let Some(buf) = self.st.buffer() {
                    let text_before_cursor = &buf.text()[0..pos as usize];
                    let lines: Vec<&str> = text_before_cursor.split('\n').collect();
                    let row = lines.len() as i32;
                    let col = lines.last().map(|line| line.len() as i32).unwrap_or(0);
                    self.saved_cursor_pos = Some((row, col));
                }
            }
            
    
            56 => 
                if let Some((row, col)) = self.saved_cursor_pos {
                    self.set_cursor_position(row, col);
                }
            }
            
    
            99 => 
        
                if let Some(mut buf) = self.st.buffer() {
                    buf.set_text("");
                }
                if let Some(mut style_buf) = self.st.style_buffer() {
                    style_buf.set_text("");
                }
                self.ansi_state.reset_attributes();
                self.saved_cursor_pos = None;
                
        
                if let Ok(mut scrollback) = self.scrollback.lock() {
                    scrollback.clear();
                }
                
        
                self.st.set_insert_position(0);
            }
            
            _ => {
        }
    }
}


