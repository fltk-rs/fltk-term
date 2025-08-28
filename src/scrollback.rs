use std::collections::VecDeque;


#[derive(Debug, Clone)]
pub struct ScrollbackLine {
    pub text: String,
    pub styles: String, 
}

impl ScrollbackLine {
    pub fn new(text: String, styles: String) -> Self {
        Self { text, styles }
    }
    
    pub fn empty() -> Self {
        Self {
            text: String::new(),
            styles: String::new(),
        }
    }
}


#[derive(Debug)]
pub struct ScrollbackBuffer {
    lines: VecDeque<ScrollbackLine>,
    max_lines: usize,
    current_scroll_offset: usize, 
}

impl ScrollbackBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            current_scroll_offset: 0,
        }
    }

    
    pub fn add_line(&mut self, line: ScrollbackLine) {
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
        
        
        
        if self.current_scroll_offset == 0 {
            
        } else {
            
        }
    }

    
    pub fn scroll_up(&mut self, lines: usize) -> bool {
        let max_scroll = self.lines.len().saturating_sub(1);
        let new_offset = (self.current_scroll_offset + lines).min(max_scroll);
        
        if new_offset != self.current_scroll_offset {
            self.current_scroll_offset = new_offset;
            true 
        } else {
            false 
        }
    }

    
    pub fn scroll_down(&mut self, lines: usize) -> bool {
        let new_offset = self.current_scroll_offset.saturating_sub(lines);
        
        if new_offset != self.current_scroll_offset {
            self.current_scroll_offset = new_offset;
            true 
        } else {
            false 
        }
    }

    
    pub fn get_visible_lines(&self, terminal_rows: usize) -> Vec<ScrollbackLine> {
        if self.lines.is_empty() {
            return vec![ScrollbackLine::empty(); terminal_rows];
        }

        let total_lines = self.lines.len();
        let start_idx = if total_lines > terminal_rows {
            total_lines - terminal_rows - self.current_scroll_offset
        } else {
            0
        };
        
        let end_idx = (start_idx + terminal_rows).min(total_lines);
        
        let mut visible_lines: Vec<ScrollbackLine> = self.lines
            .range(start_idx..end_idx)
            .cloned()
            .collect();
        
        
        while visible_lines.len() < terminal_rows {
            visible_lines.push(ScrollbackLine::empty());
        }
        
        visible_lines
    }

    
    pub fn is_scrolled(&self) -> bool {
        self.current_scroll_offset > 0
    }

    
    pub fn scroll_to_bottom(&mut self) {
        self.current_scroll_offset = 0;
    }

    
    pub fn scroll_offset(&self) -> usize {
        self.current_scroll_offset
    }

    
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    
    pub fn set_max_lines(&mut self, max_lines: usize) {
        self.max_lines = max_lines;
        
        
        while self.lines.len() > max_lines {
            self.lines.pop_front();
        }
    }

    
    pub fn clear(&mut self) {
        self.lines.clear();
        self.current_scroll_offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollback_buffer_basic() {
        let mut buffer = ScrollbackBuffer::new(3);
        
        buffer.add_line(ScrollbackLine::new("line1".to_string(), "AAAAA".to_string()));
        buffer.add_line(ScrollbackLine::new("line2".to_string(), "BBBBB".to_string()));
        buffer.add_line(ScrollbackLine::new("line3".to_string(), "CCCCC".to_string()));
        
        assert_eq!(buffer.line_count(), 3);
        assert!(!buffer.is_scrolled());
    }

    #[test]
    fn test_scrollback_buffer_overflow() {
        let mut buffer = ScrollbackBuffer::new(2);
        
        buffer.add_line(ScrollbackLine::new("line1".to_string(), "AAAAA".to_string()));
        buffer.add_line(ScrollbackLine::new("line2".to_string(), "BBBBB".to_string()));
        buffer.add_line(ScrollbackLine::new("line3".to_string(), "CCCCC".to_string()));
        
        assert_eq!(buffer.line_count(), 2);
        let visible = buffer.get_visible_lines(2);
        assert_eq!(visible[0].text, "line2");
        assert_eq!(visible[1].text, "line3");
    }

    #[test]
    fn test_scrolling() {
        let mut buffer = ScrollbackBuffer::new(5);
        
        for i in 1..=5 {
            buffer.add_line(ScrollbackLine::new(format!("line{}", i), "A".repeat(5)));
        }
        
        assert!(buffer.scroll_up(2));
        assert!(buffer.is_scrolled());
        assert_eq!(buffer.scroll_offset(), 2);
        
        assert!(buffer.scroll_down(1));
        assert_eq!(buffer.scroll_offset(), 1);
        
        buffer.scroll_to_bottom();
        assert!(!buffer.is_scrolled());
    }
}