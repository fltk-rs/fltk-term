use crate::styles::*;
use fltk::enums::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub overline: bool,
    pub inverse: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: WHITE,
            bg: BLACK,
            bold: false,
            faint: false,
            italic: false,
            underline: false,
            strikethrough: false,
            overline: false,
            inverse: false,
        }
    }
}

impl Style {
    pub fn new(bg: Color, fg: Color) -> Self {
        Self {
            fg,
            bg,
            bold: false,
            faint: false,
            italic: false,
            underline: false,
            strikethrough: false,
            overline: false,
            inverse: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Cell {
    pub fn new(ch: char, style: Style) -> Self {
        Self { ch, style }
    }
}

pub struct CellBuffer {
    lines: Vec<Vec<Cell>>, // simple grow-only buffer for now
    pub max_lines: usize,
    pub cur_style: Style,
    pub default_bg: Color,
    pub default_fg: Color,
    dirty_start: Option<usize>,
    dirty_end: Option<usize>,
    dirty_cols: Vec<Option<(usize, usize)>>,
    cursor_row: usize,
    cursor_col: usize,
    cols: usize,
    rows: usize,
}

impl CellBuffer {
    pub fn new(max_lines: usize, default_bg: Color, default_fg: Color) -> Self {
        Self {
            lines: vec![Vec::new()],
            max_lines,
            cur_style: Style {
                fg: default_fg,
                bg: default_bg,
                ..Default::default()
            },
            default_bg,
            default_fg,
            dirty_start: None,
            dirty_end: None,
            dirty_cols: vec![None],
            cursor_row: 0,
            cursor_col: 0,
            cols: 80,
            rows: 24,
        }
    }

    pub fn set_style(&mut self, style: Style) {
        self.cur_style = style;
    }

    pub fn push_char(&mut self, ch: char) {
        if let Some(line) = self.lines.last_mut() {
            let col = line.len();
            line.push(Cell::new(ch, self.cur_style));
            let idx = self.lines.len().saturating_sub(1);
            self.mark_dirty_line(idx);
            self.mark_dirty_cols(idx, col, col);
            self.cursor_row = idx;
            self.cursor_col = col + 1;
        }
    }

    pub fn newline(&mut self) {
        self.lines.push(Vec::new());
        self.dirty_cols.push(None);
        if self.lines.len() > self.max_lines {
            let overflow = self.lines.len() - self.max_lines;
            self.lines.drain(0..overflow);
            if overflow > 0 {
                if self.dirty_cols.len() >= overflow {
                    self.dirty_cols.drain(0..overflow);
                }
                self.cursor_row = self.cursor_row.saturating_sub(overflow);
            }
        }
        let idx = self.lines.len().saturating_sub(1);
        self.mark_dirty_line(idx);
        self.mark_dirty_cols(idx, 0, 0);
        self.cursor_row = idx;
        self.cursor_col = 0;
    }

    pub fn clear_line_right(&mut self) {
        if let Some(line) = self.lines.last_mut() {
            let before = line.len();
            line.clear();
            let idx = self.lines.len().saturating_sub(1);
            self.mark_dirty_line(idx);
            if before > 0 {
                self.mark_dirty_cols(idx, 0, before - 1);
            }
            self.cursor_row = idx;
            self.cursor_col = 0;
        }
    }

    pub fn snapshot(&self) -> Vec<Vec<Cell>> {
        self.lines.clone()
    }

    pub fn set_dimensions(&mut self, cols: usize, rows: usize) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
        // Keep cursor anchored to bottom region if it fell outside
        let st = self.screen_top();
        let sb = st + self.rows.saturating_sub(1);
        if self.cursor_row < st {
            self.cursor_row = st;
            self.cursor_col = 0;
        } else if self.cursor_row > sb {
            self.cursor_row = sb;
            self.cursor_col = 0;
        }
    }

    fn screen_top(&self) -> usize {
        self.lines.len().saturating_sub(self.rows)
    }

    fn screen_bottom(&self) -> usize {
        self.lines.len().saturating_sub(1).max(
            self.screen_top()
                .saturating_add(self.rows.saturating_sub(1)),
        )
    }

    fn mark_dirty_line(&mut self, idx: usize) {
        self.dirty_start = Some(self.dirty_start.map(|s| s.min(idx)).unwrap_or(idx));
        self.dirty_end = Some(self.dirty_end.map(|e| e.max(idx)).unwrap_or(idx));
    }

    fn mark_dirty_cols(&mut self, idx: usize, start: usize, end: usize) {
        if idx >= self.dirty_cols.len() {
            self.dirty_cols.resize(idx + 1, None);
        }
        self.dirty_cols[idx] = match self.dirty_cols[idx] {
            Some((s, e)) => Some((s.min(start), e.max(end))),
            None => Some((start, end)),
        };
    }

    pub fn take_dirty(&mut self) -> Option<(usize, usize)> {
        match (self.dirty_start.take(), self.dirty_end.take()) {
            (Some(s), Some(e)) if s <= e => Some((s, e)),
            _ => None,
        }
    }

    pub fn take_dirty_areas(&mut self) -> Vec<(usize, usize, usize)> {
        let mut areas = Vec::new();
        for (i, rng) in self.dirty_cols.iter_mut().enumerate() {
            if let Some((s, e)) = rng.take() {
                areas.push((i, s, e));
            }
        }
        areas
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = row;
        self.cursor_col = col;
    }

    fn ensure_row(&mut self, row: usize) {
        while self.lines.len() <= row {
            self.lines.push(Vec::new());
            self.dirty_cols.push(None);
        }
    }

    fn ensure_col(&mut self, row: usize, col: usize) {
        self.ensure_row(row);
        let line = &mut self.lines[row];
        if line.len() <= col {
            let pad = col + 1 - line.len();
            for _ in 0..pad {
                line.push(Cell::new(' ', self.cur_style));
            }
        }
    }

    pub fn write_char(&mut self, ch: char) {
        // simple DECAWM-style wrap at cols
        if self.cols > 0 && self.cursor_col >= self.cols {
            self.line_feed();
        }
        let (row, col) = (self.cursor_row, self.cursor_col);
        self.ensure_col(row, col);
        if let Some(cell) = self.lines[row].get_mut(col) {
            cell.ch = ch;
            cell.style = self.cur_style;
        }
        self.mark_dirty_line(row);
        self.mark_dirty_cols(row, col, col);
        self.cursor_col = self.cursor_col.saturating_add(1);
    }

    pub fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }

    pub fn line_feed(&mut self) {
        self.cursor_row = self.cursor_row.saturating_add(1);
        self.cursor_col = 0;
        self.ensure_row(self.cursor_row);
        self.mark_dirty_line(self.cursor_row);
    }

    pub fn move_cursor_rel(&mut self, drow: isize, dcol: isize) {
        let st = self.screen_top();
        let sb = st + self.rows.saturating_sub(1);
        let nr = (self.cursor_row as isize + drow).clamp(st as isize, sb as isize) as usize;
        let mut nc = (self.cursor_col as isize + dcol).max(0) as usize;
        // clamp col to screen cols if set
        if self.cols > 0 {
            nc = nc.min(self.cols.saturating_sub(1));
        }
        self.cursor_row = nr;
        self.cursor_col = nc;
        self.ensure_row(nr);
    }

    pub fn move_cursor_abs(&mut self, row1: usize, col1: usize) {
        // Interpret row/col as screen-relative (0-based)
        let st = self.screen_top();
        let row = st + row1.min(self.rows.saturating_sub(1));
        let col = if self.cols > 0 {
            col1.min(self.cols.saturating_sub(1))
        } else {
            col1
        };
        self.cursor_row = row;
        self.cursor_col = col;
        self.ensure_row(row);
    }

    pub fn clear_eol(&mut self) {
        self.ensure_row(self.cursor_row);
        let col = self.cursor_col;
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            if col < line.len() {
                let end = line.len() - 1;
                line.truncate(col);
                self.mark_dirty_line(self.cursor_row);
                self.mark_dirty_cols(self.cursor_row, col, end);
            }
        }
    }

    // EL 0: erase from cursor to end of line (inclusive)
    pub fn clear_eol_0(&mut self) {
        self.ensure_row(self.cursor_row);
        let col = self.cursor_col;
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            if col < line.len() {
                let end = line.len().saturating_sub(1);
                for i in col..=end {
                    if let Some(cell) = line.get_mut(i) {
                        cell.ch = ' ';
                        cell.style = self.cur_style;
                    }
                }
                self.mark_dirty_line(self.cursor_row);
                self.mark_dirty_cols(self.cursor_row, col, end);
            }
        }
    }

    // EL 1: erase from start of line to cursor (inclusive of position before the cursor)
    pub fn clear_eol_1(&mut self) {
        let row = self.cursor_row;
        self.ensure_row(row);
        let col = self.cursor_col;
        // Ensure cells exist up to cursor first to avoid borrow conflict
        self.ensure_col(row, col);
        if let Some(line) = self.lines.get_mut(row) {
            if !line.is_empty() {
                let end = col.min(line.len().saturating_sub(1));
                for i in 0..=end {
                    if let Some(cell) = line.get_mut(i) {
                        cell.ch = ' ';
                        cell.style = self.cur_style;
                    }
                }
                self.mark_dirty_line(row);
                self.mark_dirty_cols(row, 0, end);
            }
        }
    }

    // EL 2: erase entire line
    pub fn clear_eol_2(&mut self) {
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            let len = line.len();
            if len > 0 {
                for c in line.iter_mut() {
                    c.ch = ' ';
                    c.style = self.cur_style;
                }
                self.mark_dirty_line(self.cursor_row);
                self.mark_dirty_cols(self.cursor_row, 0, len.saturating_sub(1));
            }
        }
    }

    pub fn clear_ed_0(&mut self) {
        // Clear from cursor to end of screen
        self.clear_eol_0();
        let row = self.cursor_row;
        let sb = self.screen_top() + self.rows.saturating_sub(1);
        let end_row = sb.min(self.lines.len().saturating_sub(1));
        if row < end_row {
            for r in row + 1..=end_row {
                let len = self.lines[r].len();
                if len > 0 {
                    self.lines[r].clear();
                    self.mark_dirty_line(r);
                    self.mark_dirty_cols(r, 0, len.saturating_sub(1));
                }
            }
        }
    }

    pub fn clear_ed_1(&mut self) {
        // Clear from start of screen to cursor position (inclusive)
        let row = self.cursor_row;
        let st = self.screen_top();
        // clear previous lines within the visible screen entirely
        for r in st..row {
            let len = self.lines.get(r).map(|l| l.len()).unwrap_or(0);
            if len > 0 {
                if let Some(line) = self.lines.get_mut(r) {
                    line.clear();
                }
                self.mark_dirty_line(r);
                self.mark_dirty_cols(r, 0, len.saturating_sub(1));
            }
        }
        // clear current line from start to cursor col without shifting remainder
        self.ensure_row(row);
        let col = self.cursor_col;
        // ensure cells exist up to cursor before borrowing line
        self.ensure_col(row, col);
        if let Some(line) = self.lines.get_mut(row) {
            let end = col.min(line.len().saturating_sub(1));
            for i in 0..=end {
                if let Some(cell) = line.get_mut(i) {
                    cell.ch = ' ';
                    cell.style = self.cur_style;
                }
            }
            self.mark_dirty_line(row);
            if end < usize::MAX {
                self.mark_dirty_cols(row, 0, end);
            }
        }
    }

    pub fn clear_ed_2(&mut self) {
        // Clear entire screen (visible area only)
        let st = self.screen_top();
        let sb = st + self.rows.saturating_sub(1);
        let end_row = sb.min(self.lines.len().saturating_sub(1));
        for r in st..=end_row {
            let len = self.lines.get(r).map(|l| l.len()).unwrap_or(0);
            if let Some(line) = self.lines.get_mut(r) {
                if len > 0 {
                    line.clear();
                }
            }
            if len > 0 {
                self.mark_dirty_line(r);
                self.mark_dirty_cols(r, 0, len.saturating_sub(1));
            }
        }
        // place cursor at top-left of screen
        self.cursor_row = st;
        self.cursor_col = 0;
    }

    pub fn clear_scrollback(&mut self) {
        // Clear scrollback: keep only the visible screen region
        let st = self.screen_top();
        if st > 0 {
            let keep: Vec<Vec<Cell>> = self.lines.split_off(st);
            self.lines = keep;
            self.dirty_cols = vec![None; self.lines.len()];
            self.cursor_row = self.cursor_row.saturating_sub(st);
            self.mark_dirty_line(0);
            if !self.lines.is_empty() {
                let last_len = self.lines[0].len();
                self.mark_dirty_cols(0, 0, last_len.saturating_sub(1));
            }
        }
    }

    pub fn insert_blanks(&mut self, count: usize) {
        self.ensure_row(self.cursor_row);
        let col = self.cursor_col;
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            let blanks = std::iter::repeat_n(Cell::new(' ', self.cur_style), count);
            if col >= line.len() {
                line.extend(blanks);
            } else {
                line.splice(col..col, blanks);
            }
            let end = col + count;
            self.mark_dirty_line(self.cursor_row);
            self.mark_dirty_cols(self.cursor_row, col, end);
        }
    }

    pub fn delete_chars(&mut self, count: usize) {
        self.ensure_row(self.cursor_row);
        let col = self.cursor_col;
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            if col < line.len() {
                let end = (col + count).min(line.len());
                line.drain(col..end);
                self.mark_dirty_line(self.cursor_row);
                self.mark_dirty_cols(self.cursor_row, col, end.saturating_sub(1));
            }
        }
    }

    pub fn insert_lines(&mut self, count: usize) {
        let row = self.cursor_row.min(self.lines.len());
        for _ in 0..count {
            self.lines.insert(row, Vec::new());
            self.dirty_cols.insert(row, None);
        }
        // Trim to max_lines by removing from end
        if self.lines.len() > self.max_lines {
            let overflow = self.lines.len() - self.max_lines;
            for _ in 0..overflow {
                self.lines.pop();
                self.dirty_cols.pop();
            }
        }
        // Mark dirty affected range
        let end = (row + count).min(self.lines.len().saturating_sub(1));
        self.mark_dirty_line(row);
        self.mark_dirty_line(end);
    }

    pub fn delete_lines(&mut self, count: usize) {
        let row = self.cursor_row.min(self.lines.len());
        let end = (row + count).min(self.lines.len());
        if row < end {
            self.lines.drain(row..end);
            self.dirty_cols.drain(row..end);
            // push one empty line at bottom to maintain capacity if desired
            if self.lines.len() < self.max_lines {
                self.lines.push(Vec::new());
                self.dirty_cols.push(None);
            }
            self.mark_dirty_line(row);
        }
    }

    // Horizontal Tab: advance to next 8-column stop
    pub fn tab(&mut self) {
        let next = ((self.cursor_col / 8) + 1) * 8;
        if next > self.cursor_col {
            // ensure cells until next-1 exist (filled with spaces)
            self.ensure_col(self.cursor_row, next.saturating_sub(1));
            // mark dirty area
            self.mark_dirty_line(self.cursor_row);
            self.mark_dirty_cols(self.cursor_row, self.cursor_col, next.saturating_sub(1));
            self.cursor_col = next;
        }
    }

    // ECH: erase N characters from cursor, cursor does not move
    pub fn erase_chars(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let row = self.cursor_row;
        let start = self.cursor_col;
        let end = start.saturating_add(count).saturating_sub(1);
        self.ensure_col(row, end);
        if let Some(line) = self.lines.get_mut(row) {
            let last = end.min(line.len().saturating_sub(1));
            for i in start..=last {
                if let Some(cell) = line.get_mut(i) {
                    cell.ch = ' ';
                    cell.style = self.cur_style;
                }
            }
            self.mark_dirty_line(row);
            self.mark_dirty_cols(row, start, last);
        }
    }
}
