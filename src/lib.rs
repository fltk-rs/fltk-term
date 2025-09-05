#![doc = include_str!("../README.md")]
#![allow(dead_code)]
#![allow(clippy::single_match)]
#![allow(clippy::needless_doctest_main)]

use fltk::{enums::*, prelude::*, *};
use std::{
    io::{self, Write},
    str,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
mod canvas;
pub mod cell_performer;
mod cells;
mod pty;
mod styles;

pub use canvas::TermCanvas;
pub use cells::{Cell, CellBuffer, Style};
use styles::*;

const UP: &[u8] = if cfg!(not(target_os = "windows")) {
    b"\x10"
} else {
    b"\x1b[A"
};
const DOWN: &[u8] = if cfg!(not(target_os = "windows")) {
    b"\x0E"
} else {
    b"\x1b[B"
};

pub fn menu_cb(m: &mut impl MenuExt) {
    if let Ok(mpath) = m.item_pathname(None) {
        match mpath.as_str() {
            "/Copy\t" => {
                let mut term: group::Group = app::widget_from_id("term").unwrap();
                term.do_callback();
            }
            "/Paste\t" => {
                let term: group::Group = app::widget_from_id("term").unwrap();
                app::paste_text(&term);
            }
            _ => (),
        }
    }
}

pub fn init_menu(m: &mut (impl MenuExt + 'static)) {
    m.add(
        "Copy\t",
        Shortcut::Ctrl | Shortcut::Shift | 'c',
        menu::MenuFlag::Normal,
        menu_cb,
    );
    m.add(
        "Paste\t",
        Shortcut::Ctrl | Shortcut::Shift | 'v',
        menu::MenuFlag::Normal,
        menu_cb,
    );
}

/// Helper function to copy selection to clipboard
fn copy_selection_to_clipboard(
    sel_arc: &Arc<Mutex<crate::canvas::Selection>>,
    buffer: &Arc<Mutex<CellBuffer>>,
    widget_width: i32,
) {
    if let Ok(ssel) = sel_arc.lock() {
        if let (Some(mut a), Some(mut b)) = (ssel.start, ssel.end) {
            if b < a {
                std::mem::swap(&mut a, &mut b);
            }
            if let Ok(buf) = buffer.lock() {
                let pad_x = 6;
                let char_w = ((draw::width("M") as f32).ceil() as i32).max(1);
                let cols = ((widget_width - 2 * pad_x).max(1) / char_w).max(1) as usize;
                let snap = buf.snapshot();
                let mut visual: Vec<Vec<char>> = Vec::new();
                for line in snap.iter() {
                    let row: Vec<char> = line.iter().map(|c| c.ch).collect();
                    if row.is_empty() {
                        visual.push(Vec::new());
                        continue;
                    }
                    for chunk in row.chunks(cols) {
                        visual.push(chunk.to_vec());
                    }
                }
                let mut out = String::new();
                for v in a.0..=b.0 {
                    if v >= visual.len() {
                        break;
                    }
                    let line = &visual[v];
                    let start = if v == a.0 { a.1.min(line.len()) } else { 0 };
                    let end = if v == b.0 {
                        (b.1 + 1).min(line.len())
                    } else {
                        line.len()
                    };
                    if start < end {
                        for ch in &line[start..end] {
                            out.push(*ch);
                        }
                    }
                    if v != b.0 {
                        out.push('\n');
                    }
                }
                if !out.is_empty() {
                    app::copy(&out);
                    app::copy2(&out);
                }
            }
        }
    }
}

/// Helper function to scroll to bottom
fn scroll_to_bottom(scroll: &mut group::Scroll, canvas: &group::Group) {
    let view_h = scroll.h();
    let max_y = (canvas.h() - view_h).max(0);
    scroll.scroll_to(scroll.xposition(), max_y);
}

/// Helper function to paste text and scroll to bottom
fn paste_and_scroll(widget: &group::Group, scroll: &mut group::Scroll, canvas: &group::Group) {
    app::paste_text(widget);
    scroll_to_bottom(scroll, canvas);
}

/// Helper function to check if copy shortcut is pressed
fn is_copy_shortcut() -> bool {
    let key = app::event_key();
    app::event_state().contains(EventState::Ctrl)
        && app::event_state().contains(EventState::Shift)
        && key == Key::from_char('c')
}

/// Helper function to check if paste shortcut is pressed
fn is_paste_shortcut() -> bool {
    let key = app::event_key();
    (app::event_state().contains(EventState::Ctrl)
        && app::event_state().contains(EventState::Shift)
        && key == Key::from_char('v'))
        || (app::event_state().contains(EventState::Shift) && key == Key::Insert)
}

pub struct PPTerm {
    scroll: group::Scroll,
    canvas: TermCanvas,
    pty: Option<pty::PtyHandles>,
    shutdown_flag: Arc<AtomicBool>,
    buffer: Arc<Mutex<CellBuffer>>,
    cols: u16,
    rows: u16,
    auto_follow: Arc<Mutex<bool>>,
}

impl Default for PPTerm {
    fn default() -> Self {
        PPTerm::new(0, 0, 0, 0, None)
    }
}

impl PPTerm {
    /// Internal constructor that allows specifying initial terminal cols/rows.
    #[allow(clippy::too_many_arguments)]
    fn new_with_cols_rows_internal<L: Into<Option<&'static str>>>(
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: L,
        init_cols: u16,
        init_rows: u16,
        max_lines: usize,
    ) -> Self {
        let mut scroll =
            group::Scroll::new(x, y, w, h, label).with_type(group::ScrollType::Vertical);
        scroll.set_id("term_group");
        let buffer = Arc::new(Mutex::new(CellBuffer::new(max_lines, BLACK, WHITE)));
        let mut canvas = TermCanvas::new(scroll.x(), scroll.y(), w, h, None);
        canvas.set_id("term");
        canvas.set_buffer(buffer.clone());
        canvas.set_scroll(scroll.clone());
        canvas.start_blink(0.6);
        canvas.end();
        let mut m = menu::MenuButton::default()
            .with_type(menu::MenuButtonType::Popup3)
            .with_id("pop2");
        init_menu(&mut m);
        scroll.end();

        // Compute approximate monospace metrics
        draw::set_font(Font::Courier, 14);
        let sample = "MMMMMMMMMM";
        let (sw, sh) = draw::measure(sample, false);
        let char_w = (sw as f32 / 10.0).ceil() as i32;
        let line_h = sh.max(14);

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let handles = pty::start(buffer.clone(), init_cols, init_rows, shutdown_flag.clone());
        let master_pty_arc_opt = handles.as_ref().map(|h| h.master_pty.clone());

        // React to outer widget resize: update canvas width and PTY cols/rows
        scroll.resize_callback({
            let mut canvas = canvas.clone();
            let master_pty_cb = master_pty_arc_opt.clone();
            let mut menu = m.clone();
            move |_, x, y, w, h| {
                // Important: don't call scroll.resize() here to avoid recursive callbacks.
                // Only adjust child canvas width; height is managed by the periodic updater.
                menu.resize(x, y, w, h);
                canvas.set_size(w, canvas.h());
                let pad_x = 6;
                let pad_y = 4;
                let cols = ((w - 2 * pad_x).max(char_w) / char_w).max(10) as u16;
                let rows = ((h - pad_y).max(line_h) / line_h).max(3) as u16;
                if let Some(ref pty) = master_pty_cb {
                    let _ = pty::resize_pty(pty, cols, rows);
                }
            }
        });

        // Keyboard input -> PTY
        if let Some(ref handles_ref) = handles {
            let selection_arc = canvas.selection_handle();
            let buffer_for_copy = buffer.clone();
            let mut scroll_for_input = scroll.clone();
            let canvas_for_input = canvas.clone();
            canvas.set_callback({
                let sel_arc = selection_arc.clone();
                let buf = buffer.clone();
                move |g| {
                    copy_selection_to_clipboard(&sel_arc, &buf, g.w());
                }
            });
            canvas.handle({
                let writer = handles_ref.writer.clone();
                move |t, ev| match ev {
                    Event::Push => {
                        if app::event_button() == 1 {
                            t.take_focus().ok();
                            let (mx, my) = (app::event_x(), app::event_y());
                            // begin selection (account for scroll viewport + offset)
                            let line_h = draw::height().max(14);
                            let char_w = ((draw::width("M") as f32).ceil() as i32).max(1);
                            let pad_x = 6;
                            let view_top = scroll_for_input.y();
                            let yoff = scroll_for_input.yposition();
                            let row_view = ((my - view_top).max(0) / line_h) as usize;
                            let row = row_view + (yoff / line_h).max(0) as usize;
                            let col = ((mx - t.x() - pad_x).max(0) / char_w) as usize;
                            if let Ok(mut sel) = selection_arc.lock() {
                                sel.start = Some((row, col));
                                sel.end = Some((row, col));
                            }
                            t.redraw();
                            true
                        } else {
                            false
                        }
                    }
                    Event::KeyDown => {
                        if is_copy_shortcut() {
                            copy_selection_to_clipboard(&selection_arc, &buffer_for_copy, t.w());
                            return true;
                        }
                        if is_paste_shortcut() {
                            paste_and_scroll(t, &mut scroll_for_input, &canvas_for_input);
                            return true;
                        }
                        let key = app::event_key();
                        let mods = app::event_state();
                        let has_shift = mods.contains(EventState::Shift);
                        let has_alt = mods.contains(EventState::Alt);
                        let has_ctrl = mods.contains(EventState::Ctrl);
                        let send = |bytes: &[u8]| {
                            if let Ok(mut w) = writer.lock() {
                                if has_alt {
                                    let _ = w.write_all(b"\x1b"); // Alt prefix
                                }
                                let _ = w.write_all(bytes);
                            }
                        };
                        let arrow_with_mods = |letter: u8| -> Vec<u8> {
                            let mut v = Vec::new();
                            let mut m = 1; // base
                            if has_shift {
                                m += 1;
                            } // 2
                            if has_alt {
                                m += 2;
                            } // 3
                            if has_ctrl {
                                m += 4;
                            } // 5
                            if m == 1 {
                                v.extend_from_slice(&[0x1b, b'[', letter]);
                            } else {
                                v.extend_from_slice(b"\x1b[1;");
                                v.extend_from_slice(m.to_string().as_bytes());
                                v.push(letter);
                            }
                            v
                        };
                        match key {
                            #[cfg(windows)]
                            Key::BackSpace => {
                                if let Ok(mut w) = writer.lock() {
                                    let _ = w.write_all(b"\x7f");
                                }
                            }
                            Key::Up => {
                                let seq = if has_shift || has_alt || has_ctrl {
                                    arrow_with_mods(b'A')
                                } else {
                                    UP.to_vec()
                                };
                                send(&seq);
                            }
                            Key::Down => {
                                let seq = if has_shift || has_alt || has_ctrl {
                                    arrow_with_mods(b'B')
                                } else {
                                    DOWN.to_vec()
                                };
                                send(&seq);
                            }
                            Key::Left => {
                                let seq = arrow_with_mods(b'D');
                                send(&seq);
                            }
                            Key::Right => {
                                let seq = arrow_with_mods(b'C');
                                send(&seq);
                            }
                            // Prefer VT220-style for unmodified; many shells expect 1~/4~
                            Key::Home => {
                                if has_shift || has_alt || has_ctrl {
                                    let seq = arrow_with_mods(b'H');
                                    send(&seq);
                                } else {
                                    send(b"\x1b[1~");
                                }
                            }
                            Key::End => {
                                if has_shift || has_alt || has_ctrl {
                                    let seq = arrow_with_mods(b'F');
                                    send(&seq);
                                } else {
                                    send(b"\x1b[4~");
                                }
                            }
                            Key::PageUp => {
                                send(b"\x1b[5~");
                            }
                            Key::PageDown => {
                                send(b"\x1b[6~");
                            }
                            Key::Insert => {
                                send(b"\x1b[2~");
                            }
                            Key::Delete => {
                                send(b"\x1b[3~");
                            }
                            Key::Enter => {
                                send(b"\r");
                            }
                            Key::F1 => {
                                send(b"\x1bOP");
                            }
                            Key::F2 => {
                                send(b"\x1bOQ");
                            }
                            Key::F3 => {
                                send(b"\x1bOR");
                            }
                            Key::F4 => {
                                send(b"\x1bOS");
                            }
                            Key::F5 => {
                                send(b"\x1b[15~");
                            }
                            Key::F6 => {
                                send(b"\x1b[17~");
                            }
                            Key::F7 => {
                                send(b"\x1b[18~");
                            }
                            Key::F8 => {
                                send(b"\x1b[19~");
                            }
                            Key::F9 => {
                                send(b"\x1b[20~");
                            }
                            Key::F10 => {
                                send(b"\x1b[21~");
                            }
                            Key::F11 => {
                                send(b"\x1b[23~");
                            }
                            Key::F12 => {
                                send(b"\x1b[24~");
                            }
                            _ => {
                                let txt = app::event_text();
                                if !txt.is_empty() {
                                    send(txt.as_bytes());
                                }
                            }
                        }
                        // Auto-scroll to bottom on any user key input
                        scroll_to_bottom(&mut scroll_for_input, &canvas_for_input);
                        true
                    }
                    Event::Drag => {
                        let (mx, my) = (app::event_x(), app::event_y());
                        // Update selection end (account for scroll viewport + offset)
                        let line_h = draw::height().max(14);
                        let char_w = ((draw::width("M") as f32).ceil() as i32).max(1);
                        let pad_x = 6;
                        let view_top = scroll_for_input.y();
                        let yoff = scroll_for_input.yposition();
                        let row_view = ((my - view_top).max(0) / line_h) as usize;
                        let row = row_view + (yoff / line_h).max(0) as usize;
                        let col = ((mx - t.x() - pad_x).max(0) / char_w) as usize;
                        if let Ok(mut sel) = selection_arc.lock() {
                            sel.end = Some((row, col));
                        }
                        // Auto-scroll during selection when dragging beyond the widget edges
                        let edge = 14; // px threshold
                                       // Monospace line height
                                       // line_h computed above
                        let mut new_y = scroll_for_input.yposition();
                        // Use viewport edges to decide scroll
                        let vt = scroll_for_input.y();
                        let vb = vt + scroll_for_input.h();
                        // Use a larger step so the pointer remains inside widget when dragging upward
                        if my <= vt + edge {
                            new_y = new_y.saturating_sub(line_h * 3);
                        } else if my >= vb - edge {
                            new_y = new_y.saturating_add(line_h * 2);
                        }
                        if new_y != scroll_for_input.yposition() {
                            let max_y = (canvas_for_input.h() - scroll_for_input.h()).max(0);
                            let new_y = new_y.clamp(0, max_y);
                            scroll_for_input.scroll_to(scroll_for_input.xposition(), new_y);
                        }
                        t.redraw();
                        true
                    }
                    Event::Released => {
                        // Middle-click paste (X11-style) or right-click menu action
                        if app::event_button() == 2 {
                            app::paste_text(t);
                        } else if app::event_button() == 3 {
                            m.popup();
                        }
                        true
                    }
                    Event::Paste => {
                        // Prefer event_clipboard() for portability; fallback to event_text()
                        let mut pasted = String::new();
                        if let Some(cb) = app::event_clipboard() {
                            match cb {
                                app::ClipboardEvent::Text(s) => pasted = s,
                                app::ClipboardEvent::Image(_) => {}
                            }
                        }
                        if pasted.is_empty() {
                            pasted = app::event_text();
                        }
                        if !pasted.is_empty() {
                            if let Ok(mut w) = writer.lock() {
                                let _ = w.write_all(pasted.as_bytes());
                            }
                        }
                        // Auto-scroll after paste
                        scroll_to_bottom(&mut scroll_for_input, &canvas_for_input);
                        true
                    }
                    Event::Focus => true,
                    Event::Unfocus => true,
                    _ => false,
                }
            });
        }

        // Periodic layout + resize updater
        let mut canvas_clone = canvas.clone();
        let buffer_clone = buffer.clone();
        let master_pty_clone = master_pty_arc_opt.clone();
        let mut scroll_clone = scroll.clone();
        let auto_follow_flag = Arc::new(Mutex::new(true));
        let last_vlines = Arc::new(Mutex::new(0usize));
        let last_vlines_cl = last_vlines.clone();
        let auto_follow_flag_cl = auto_follow_flag.clone();
        app::add_timeout3(0.05, move |h| {
            // Desired content height based on buffer lines and dirty region extraction
            let mut _had_dirty = false;
            let (line_count, _, _) = if let Ok(mut buf) = buffer_clone.lock() {
                let snap = buf.snapshot();
                let cols = ((canvas_clone.w() - 12).max(char_w) / char_w).max(1) as usize;
                let mut vlines = 0usize;
                for line in snap.iter() {
                    let len = line.len().max(1);
                    vlines += len.div_ceil(cols); // ceil div
                }
                // Compute dirty rectangles from per-line column ranges
                let mut dirty = Vec::new();
                for (li, cs, ce) in buf.take_dirty_areas() {
                    if li >= snap.len() {
                        continue;
                    }
                    let line = &snap[li];
                    let len = line.len();
                    let cs = cs.min(len);
                    let ce = ce.min(len.saturating_sub(1));
                    // visual rows before this line
                    let vis_before = snap
                        .iter()
                        .take(li)
                        .map(|l| l.len().max(1).div_ceil(cols))
                        .sum::<usize>();
                    let start_seg = cs / cols;
                    let end_seg = (ce / cols).max(start_seg);
                    let y = (vis_before + start_seg) as i32 * line_h + 2;
                    let hrect = ((end_seg - start_seg + 1) as i32 * line_h).max(line_h);
                    let start_col = (cs % cols) as i32;
                    let end_col = ((ce % cols) as i32).max(start_col);
                    let x = 6 + start_col * char_w;
                    let w = ((end_col - start_col + 1) * char_w).max(char_w);
                    dirty.push((x, y, w, hrect));
                }
                if !dirty.is_empty() {
                    _had_dirty = true;
                }
                (vlines, cols as u16, Some(dirty))
            } else {
                (0usize, 80u16, None)
            };
            let pad_y = 4;
            let desired_h = (line_count as i32 * line_h + pad_y).max(scroll_clone.h());
            // Resize canvas content to match buffer
            canvas_clone.set_size(scroll_clone.w(), desired_h);

            // Compute cols/rows from viewport size (NOT content height) and resize PTY
            let pad_x = 6;
            let cols = ((scroll_clone.w() - 2 * pad_x).max(char_w) / char_w).max(10) as u16;
            let rows = ((scroll_clone.h() - pad_y).max(line_h) / line_h).max(3) as u16;
            if let Some(ref pty) = master_pty_clone {
                let _ = pty::resize_pty(pty, cols, rows);
            }
            // Keep buffer aware of the terminal grid size for CUP/ED/EL semantics
            if let Ok(mut buf) = buffer_clone.lock() {
                buf.set_dimensions(cols as usize, rows as usize);
            }

            // Auto-follow policy: only on new output and if already at bottom
            if *auto_follow_flag_cl.lock().unwrap() {
                let mut new_output = false;
                if let Ok(mut prev) = last_vlines_cl.lock() {
                    if line_count > *prev {
                        new_output = true;
                    }
                    *prev = line_count;
                }
                // Also follow on in-place updates (e.g., CR progress) by watching dirty flags
                if new_output {
                    let view_h = scroll_clone.h();
                    let max_y = (canvas_clone.h() - view_h).max(0);
                    scroll_clone.scroll_to(scroll_clone.xposition(), max_y);
                }
            }

            // For correctness, prefer full redraw; dirty rects available for future optimization.
            canvas_clone.redraw();
            app::repeat_timeout3(0.05, h);
        });

        Self {
            scroll,
            canvas,
            pty: handles,
            shutdown_flag,
            buffer,
            cols: init_cols,
            rows: init_rows,
            auto_follow: auto_follow_flag,
        }
    }

    /// Create a new canvas terminal with default dimensions (80x24).
    pub fn new<L: Into<Option<&'static str>>>(x: i32, y: i32, w: i32, h: i32, label: L) -> Self {
        Self::new_with_cols_rows_internal(x, y, w, h, label, 80, 24, 2000)
    }

    /// Create a new canvas terminal with explicit terminal dimensions (cols x rows).
    pub fn new_with_dims<L: Into<Option<&'static str>>>(
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: L,
        cols: u16,
        rows: u16,
    ) -> Self {
        Self::new_with_cols_rows_internal(x, y, w, h, label, cols, rows, 2000)
    }

    /// Convenience: construct with only (cols, rows). Position/size can be
    /// set later via standard FLTK `WidgetExt` methods like `size_of_parent()`.
    pub fn with_dimensions(cols: u16, rows: u16) -> Self {
        Self::new_with_cols_rows_internal(0, 0, 0, 0, None, cols, rows, 2000)
    }

    /// Convenience: construct with (cols, rows, scrollback_lines).
    pub fn with_dimensions_and_scrollback(cols: u16, rows: u16, scrollback_lines: usize) -> Self {
        Self::new_with_cols_rows_internal(0, 0, 0, 0, None, cols, rows, scrollback_lines)
    }

    pub fn widget(&self) -> &group::Scroll {
        &self.scroll
    }
    pub fn redraw(&mut self) {
        self.canvas.redraw();
    }
    pub fn buffer(&self) -> Arc<Mutex<CellBuffer>> {
        self.buffer.clone()
    }
    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
    }
    pub fn set_auto_follow(&mut self, enabled: bool) {
        if let Ok(mut v) = self.auto_follow.lock() {
            *v = enabled;
        }
    }
    pub fn auto_follow_handle(&self) -> Arc<Mutex<bool>> {
        self.auto_follow.clone()
    }

    /// Current column count reported by the widget.
    pub fn cols(&self) -> u16 {
        self.cols
    }
    /// Current row count reported by the widget.
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Write raw bytes to the underlying PTY.
    pub fn write_all(&self, s: &[u8]) -> Result<(), io::Error> {
        if let Some(ref pty) = &self.pty {
            match pty.writer.lock() {
                Ok(mut w) => w.write_all(s),
                Err(_) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to acquire writer lock",
                )),
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "No writer available",
            ))
        }
    }

    /// Manually set terminal cell dimensions (cols x rows) and resize the PTY.
    /// Note: the periodic layout updater will continue to sync PTY size to the
    /// current widget size; this method is most useful for initial sizing or
    /// when external code coordinates widget resizing alongside PTY resize.
    pub fn set_dims(&mut self, cols: u16, rows: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.cols = cols;
        self.rows = rows;
        if let Some(ref p) = self.pty {
            pty::resize_pty(&p.master_pty, cols, rows)
        } else {
            Ok(())
        }
    }
}

impl Drop for PPTerm {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(pty) = self.pty.take() {
            // Use a timeout to avoid hanging on close
            std::thread::spawn(move || {
                let _ = pty.thread_handle.join();
            });
        }
    }
}

fltk::widget_extends!(PPTerm, group::Scroll, scroll);

fn canvas_sel_mouse_to_vpos(t: &group::Group, mx: i32, my: i32) -> (usize, usize) {
    let line_h = draw::height().max(14);
    let char_w = ((draw::width("M") as f32).ceil() as i32).max(1);
    let pad_x = 6;
    let col = ((mx - t.x() - pad_x).max(0) / char_w) as usize;
    let row = ((my - t.y()).max(0) / line_h) as usize;
    (row, col)
}
