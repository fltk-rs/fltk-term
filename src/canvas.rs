use crate::cells::CellBuffer;
use fltk::app;
use fltk::{
    draw,
    enums::{Align, Color, Font, FrameType},
    group,
    prelude::*,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Default)]
pub struct Selection {
    pub start: Option<(usize, usize)>,
    pub end: Option<(usize, usize)>,
}

pub struct TermCanvas {
    f: group::Group,
    buffer: Arc<Mutex<CellBuffer>>,
    scroll: Option<group::Scroll>,
    blink: Arc<Mutex<bool>>,
    selection: Arc<Mutex<Selection>>,
}

impl TermCanvas {
    pub fn new<L: Into<Option<&'static str>>>(x: i32, y: i32, w: i32, h: i32, label: L) -> Self {
        let mut f = group::Group::new(x, y, w, h, label);
        f.set_frame(FrameType::FlatBox);

        Self {
            f,
            buffer: Arc::new(Mutex::new(CellBuffer::new(2000))),
            scroll: None,
            blink: Arc::new(Mutex::new(true)),
            selection: Arc::new(Mutex::new(Selection::default())),
        }
    }

    pub fn set_size(&mut self, w: i32, h: i32) {
        self.f.set_size(w, h);
    }

    pub fn widget(&self) -> &group::Group {
        &self.f
    }

    pub fn set_buffer(&mut self, buffer: Arc<Mutex<CellBuffer>>) {
        self.buffer = buffer.clone();
        let blink_state = self.blink.clone();
        let selection = self.selection.clone();
        // Rebind draw closure to capture buffer and optional scroll.
        self.f.draw(move |f| {
            let x = f.x();
            let y = f.y();
            let w = f.w();
            let h = f.h();

            draw::set_draw_color(Color::from_rgb(0, 0, 0));
            draw::draw_rectf(x, y, w, h);
            let mut font = Font::Courier;
            let font_size = 14;
            draw::set_font(font, font_size);
            let line_h = draw::height().max(14);
            let pad_x = 6;
            let mut yy = y + line_h;
            let char_w = ((draw::width("M") as f32).ceil() as i32).max(1);
            let cols = ((w - 2 * pad_x).max(1)) / char_w.max(1);

            // Snapshot selection range (visual coords)
            let sel = {
                let se = selection.lock().ok();
                let (s, e) = if let Some(ref g) = se {
                    (g.start, g.end)
                } else {
                    (None, None)
                };
                match (s, e) {
                    (Some(mut a), Some(mut b)) => {
                        if b < a {
                            std::mem::swap(&mut a, &mut b);
                        }
                        Some((a, b))
                    }
                    _ => None,
                }
            };

            if let Ok(buf) = buffer.lock() {
                let mut vline_idx: usize = 0;
                for line in buf.snapshot().iter() {
                    // wrap-aware iteration
                    let total_cols = line.len() as i32;
                    let mut col_used = 0i32;
                    let mut run: String = String::new();
                    let mut cur_fg = Color::from_rgb(255, 255, 255);
                    let mut cur_bg = Color::from_rgb(0, 0, 0);
                    let mut cur_bold = false;
                    let mut cur_underline = false;
                    let mut first = true;
                    let mut xx = x + pad_x;
                    let mut run_start_col: i32 = 0; // visual column where current run starts
                                                    // draw selection outline for this visual line
                    if let Some(((ls, cs), (le, ce))) = sel {
                        if vline_idx >= ls && vline_idx <= le {
                            let start_col = if vline_idx == ls { cs as i32 } else { 0 };
                            let end_col = if vline_idx == le { ce as i32 } else { cols - 1 };
                            if end_col >= start_col {
                                let sx = x + pad_x + start_col * char_w;
                                let sw = (end_col - start_col + 1) * char_w;
                                draw::set_draw_color(Color::from_rgb(60, 90, 160));
                                draw::draw_rect(sx, yy - line_h, sw, line_h);
                            }
                        }
                    }
                    let mut draw_run = |run: &str,
                                        fg: Color,
                                        bg: Color,
                                        bold: bool,
                                        underline: bool,
                                        xx: &mut i32,
                                        yy: i32,
                                        w: i32,
                                        col_pos: i32,
                                        vline: usize| {
                        if run.is_empty() {
                            return;
                        }
                        if *xx == x + pad_x {
                            // clear whole visual line background at start of segment
                            draw::set_draw_color(Color::from_rgb(0, 0, 0));
                            draw::draw_rectf(x + pad_x, yy - line_h, w - 2 * pad_x, line_h);
                        }
                        let (tw, _th) = draw::measure(run, false);
                        // background for this run
                        draw::set_draw_color(bg);
                        draw::draw_rectf(*xx, yy - line_h, tw, line_h);
                        // selection overlay (behind text)
                        if let Some(((ls, cs), (le, ce))) = sel {
                            if vline >= ls && vline <= le {
                                let sel_start = if vline == ls { cs as i32 } else { 0 };
                                let sel_end = if vline == le { ce as i32 } else { cols - 1 };
                                let run_cols = run.chars().count() as i32;
                                let overlap_start = sel_start.max(col_pos);
                                let overlap_end = sel_end.min(col_pos + run_cols - 1);
                                if overlap_end >= overlap_start {
                                    let sx = x + pad_x + overlap_start * char_w;
                                    let sw = (overlap_end - overlap_start + 1) * char_w;
                                    draw::set_draw_color(Color::from_rgb(30, 80, 160));
                                    draw::draw_rectf(sx, yy - line_h, sw, line_h);
                                }
                            }
                        }
                        // font weight
                        font = if bold {
                            Font::CourierBold
                        } else {
                            Font::Courier
                        };
                        draw::set_draw_color(fg);
                        draw::draw_text2(run, *xx, yy - line_h, w - *xx, line_h, Align::Left);
                        if underline {
                            let underline_y = yy - 2;
                            draw::set_draw_color(fg);
                            draw::draw_line(*xx, underline_y, *xx + tw, underline_y);
                        }
                        *xx += tw;
                    };

                    for c in line.iter() {
                        let (fg, bg) = (c.style.fg, c.style.bg);
                        let bold = c.style.bold;
                        let underline = c.style.underline;
                        let (fg_eff, bg_eff) = if c.style.inverse { (bg, fg) } else { (fg, bg) };
                        if first
                            || fg_eff != cur_fg
                            || bg_eff != cur_bg
                            || bold != cur_bold
                            || underline != cur_underline
                        {
                            if !first {
                                draw_run(
                                    &run,
                                    cur_fg,
                                    cur_bg,
                                    cur_bold,
                                    cur_underline,
                                    &mut xx,
                                    yy,
                                    w,
                                    run_start_col,
                                    vline_idx,
                                );
                                run.clear();
                            }
                            cur_fg = fg_eff;
                            cur_bg = bg_eff;
                            cur_bold = bold;
                            cur_underline = underline;
                            first = false;
                            // New run starts at current visual column
                            run_start_col = (col_used % cols) as i32;
                        }
                        run.push(c.ch);
                        col_used += 1;
                        // soft-wrap when reaching cols
                        if cols > 0 && col_used % cols == 0 {
                            // flush current run and move to next visual line
                            draw_run(
                                &run,
                                cur_fg,
                                cur_bg,
                                cur_bold,
                                cur_underline,
                                &mut xx,
                                yy,
                                w,
                                run_start_col,
                                vline_idx,
                            );
                            run.clear();
                            yy += line_h;
                            vline_idx += 1;
                            run_start_col = 0;
                            xx = x + pad_x;
                            if yy > y + h {
                                break;
                            }
                        }
                    }
                    if !run.is_empty() && yy <= y + h {
                        draw_run(
                            &run,
                            cur_fg,
                            cur_bg,
                            cur_bold,
                            cur_underline,
                            &mut xx,
                            yy,
                            w,
                            run_start_col,
                            vline_idx,
                        );
                    }
                    if total_cols == 0 {
                        // clear empty visual line
                        draw::set_draw_color(Color::from_rgb(0, 0, 0));
                        draw::draw_rectf(x + pad_x, yy - line_h, w - 2 * pad_x, line_h);
                        yy += line_h;
                        vline_idx += 1;
                    } else if total_cols % cols != 0 {
                        yy += line_h;
                        vline_idx += 1;
                    }
                    if yy > y + h {
                        break;
                    }
                }
                // Cursor rendering (blinking block)
                if let Ok(b) = blink_state.lock() {
                    if *b {
                        let (crow, ccol) = buf.cursor();
                        // Map to visual position
                        let cols = ((w - 2 * pad_x).max(1))
                            / ((draw::width("M") as f32).ceil() as i32).max(1);
                        let before_lines = buf
                            .snapshot()
                            .iter()
                            .take(crow)
                            .map(|l| (l.len().max(1) as i32 + cols - 1) / cols)
                            .sum::<i32>();
                        let seg = (ccol as i32) / cols;
                        let col = (ccol as i32) % cols;
                        let cx = x + pad_x + col * ((draw::width("M") as f32).ceil() as i32);
                        let cy = y + line_h + (before_lines + seg) * line_h;
                        if cy - line_h >= y && cy <= y + h {
                            draw::set_draw_color(Color::from_rgb(255, 255, 255));
                            draw::draw_rectf(
                                cx,
                                cy - line_h,
                                (draw::width("M") as f32).ceil() as i32,
                                line_h,
                            );
                        }
                    }
                }
                return;
            }

            // Fallback
            draw::set_draw_color(Color::from_rgb(255, 255, 255));
        });
    }

    pub fn set_scroll(&mut self, scroll: group::Scroll) {
        self.scroll = Some(scroll.clone());
        // If buffer already attached, rebind draw closure to capture scroll
        let buf = self.buffer.clone();
        self.set_buffer(buf);
    }

    pub fn start_blink(&mut self, interval: f64) {
        let blink_state = self.blink.clone();
        let mut frame = self.f.clone();
        app::add_timeout3(interval, move |h| {
            if let Ok(mut b) = blink_state.lock() {
                *b = !*b;
            }
            frame.redraw();
            app::repeat_timeout3(interval, h);
        });
    }

    fn mouse_to_vpos(&self, x: i32, y: i32) -> (usize, usize) {
        let line_h = draw::height().max(14);
        let char_w = ((draw::width("M") as f32).ceil() as i32).max(1);
        let pad_x = 6;
        let col = ((x - self.f.x() - pad_x).max(0) / char_w) as usize;
        let row = ((y - self.f.y()).max(0) / line_h) as usize;
        (row, col)
    }

    pub fn begin_selection_at(&mut self, x: i32, y: i32) {
        let vpos = self.mouse_to_vpos(x, y);
        if let Ok(mut sel) = self.selection.lock() {
            sel.start = Some(vpos);
            sel.end = Some(vpos);
        }
        self.f.redraw();
    }

    pub fn update_selection_at(&mut self, x: i32, y: i32) {
        let vpos = self.mouse_to_vpos(x, y);
        if let Ok(mut sel) = self.selection.lock() {
            sel.end = Some(vpos);
        }
        self.f.redraw();
    }

    pub fn clear_selection(&mut self) {
        if let Ok(mut sel) = self.selection.lock() {
            sel.start = None;
            sel.end = None;
        }
        self.f.redraw();
    }

    pub fn copy_selection_to_clipboard(&self) {
        let buf_arc = self.buffer.clone();
        if let Ok(buf) = buf_arc.lock() {
            let snap = buf.snapshot();
            let char_w = ((draw::width("M") as f32).ceil() as i32).max(1);
            let pad_x = 6;
            let cols = ((self.f.w() - 2 * pad_x).max(1) / char_w).max(1) as usize;
            // Build visual lines
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
            let (s, e) = if let Ok(sel) = self.selection.lock() {
                (sel.start, sel.end)
            } else {
                (None, None)
            };
            if let (Some(mut a), Some(mut b)) = (s, e) {
                if b < a {
                    std::mem::swap(&mut a, &mut b);
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
                }
            }
        };
    }

    #[allow(clippy::type_complexity)]
    pub fn selection_handle(&self) -> Arc<Mutex<Selection>> {
        self.selection.clone()
    }

    pub fn buffer_arc(&self) -> Arc<Mutex<CellBuffer>> {
        self.buffer.clone()
    }
}

fltk::widget_extends!(TermCanvas, group::Group, f);
