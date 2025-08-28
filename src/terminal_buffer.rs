use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub style: char,
}

impl Cell {
    pub fn new(ch: char, style: char) -> Self {
        Self { ch, style }
    }
    
    pub fn empty() -> Self {
        Self { ch: ' ', style: 'A' }
    }
}

#[derive(Debug)]
pub struct TerminalBuffer {

    rows: usize,
    cols: usize,
    

    screen: Vec<Vec<Cell>>,
    

    cursor_row: usize,
    cursor_col: usize,
    

    scrollback: VecDeque<Vec<Cell>>,
    max_scrollback: usize,
}

impl TerminalBuffer {
    pub fn new(rows: usize, cols: usize, max_scrollback: usize) -> Self {
        let mut screen = Vec::with_capacity(rows);
        for _ in 0..rows {
            let mut row = Vec::with_capacity(cols);
            for _ in 0..cols {
                row.push(Cell::empty());
            }
            screen.push(row);
        }
        
        Self {
            rows,
            cols,
            screen,
            cursor_row: 0,
            cursor_col: 0,
            scrollback: VecDeque::with_capacity(max_scrollback),
            max_scrollback,
        }
    }
    
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
    
        let cursor_row_ratio = self.cursor_row as f64 / self.rows as f64;
        
    
        self.rows = new_rows;
        self.cols = new_cols;
        
        self.screen.clear();
        for _ in 0..new_rows {
            let mut row = Vec::with_capacity(new_cols);
            for _ in 0..new_cols {
                row.push(Cell::empty());
            }
            self.screen.push(row);
        }
        
    
        self.cursor_row = ((cursor_row_ratio * new_rows as f64) as usize).min(new_rows - 1);
        self.cursor_col = self.cursor_col.min(new_cols - 1);
    }
    
    pub fn put_char(&mut self, ch: char, style: char) {
    
        match ch {
            '\n' => {
                self.newline();
                return;
            }
            '\r' => {
                self.cursor_col = 0;
                return;
            }
            '\t' => {
            
                let next_tab = (self.cursor_col + 8) & !7;
                self.cursor_col = next_tab.min(self.cols - 1);
                return;
            }
            _ => {}
        }
        
    
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.screen[self.cursor_row][self.cursor_col] = Cell::new(ch, style);
        }
        
    
        self.cursor_col += 1;
        if self.cursor_col >= self.cols {
            self.newline();
        }
    }
    
    fn newline(&mut self) {
        self.cursor_col = 0;
        self.cursor_row += 1;
        
        if self.cursor_row >= self.rows {
        
            let top_line = self.screen.remove(0);
            self.scrollback.push_back(top_line);
            
        
            if self.scrollback.len() > self.max_scrollback {
                self.scrollback.pop_front();
            }
            
        
            let mut new_row = Vec::with_capacity(self.cols);
            for _ in 0..self.cols {
                new_row.push(Cell::empty());
            }
            self.screen.push(new_row);
            
            self.cursor_row = self.rows - 1;
        }
    }
    
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.screen[self.cursor_row][self.cursor_col] = Cell::empty();
        }
    }
    
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = row.min(self.rows - 1);
        self.cursor_col = col.min(self.cols - 1);
    }
    
    pub fn move_cursor(&mut self, row_delta: i32, col_delta: i32) {
        let new_row = (self.cursor_row as i32 + row_delta).max(0).min(self.rows as i32 - 1) as usize;
        let new_col = (self.cursor_col as i32 + col_delta).max(0).min(self.cols as i32 - 1) as usize;
        self.set_cursor(new_row, new_col);
    }
    
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }
    
    pub fn clear_line(&mut self, line: usize) {
        if line < self.rows {
            for col in 0..self.cols {
                self.screen[line][col] = Cell::empty();
            }
        }
    }
    
    pub fn clear_screen(&mut self) {
        for row in 0..self.rows {
            self.clear_line(row);
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }
    
    pub fn render_to_string(&self) -> (String, String) {
        let mut text = String::new();
        let mut styles = String::new();
        
        for (row_idx, row) in self.screen.iter().enumerate() {
            if row_idx > 0 {
                text.push('\n');
                styles.push('\n');
            }
            
            for cell in row {
                text.push(cell.ch);
                styles.push(cell.style);
            }
        }
        
        (text, styles)
    }
    
    pub fn dimensions(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
}