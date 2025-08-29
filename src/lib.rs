#![doc = include_str!("../README.md")]

#![allow(dead_code)]
#![allow(clippy::single_match)]

use fltk::{enums::*, prelude::*, *};
use std::{
    io::{self, Write},
    str,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}, mpsc},
    thread::{self, JoinHandle},
};
use portable_pty::MasterPty;
mod ansi;
mod pty;
mod scrollback;
mod styles;
mod vte_parser;

use scrollback::ScrollbackBuffer;

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

pub(crate) struct VteParser {
    ch: char,
    st: text::TextDisplay,
    sbuf: text::TextBuffer,
    temp_s: String,
    temp_b: String,
    scrollback: Arc<Mutex<ScrollbackBuffer>>,
    current_line: String,
    current_styles: String,
    ansi_state: ansi::AnsiState,
    saved_cursor_pos: Option<(i32, i32)>,
    terminal_rows: u16,
}

impl VteParser {
    pub fn new(st: text::TextDisplay, sbuf: text::TextBuffer, scrollback: Arc<Mutex<ScrollbackBuffer>>, terminal_rows: u16) -> Self {
        let ansi_state = ansi::AnsiState::new();
        let ch = ansi_state.get_style_char();
        Self {
            ch,
            st,
            sbuf,
            temp_s: String::new(),
            temp_b: String::new(),
            scrollback,
            current_line: String::new(),
            current_styles: String::new(),
            ansi_state,
            saved_cursor_pos: None,
            terminal_rows,
        }
    }
    pub fn myprint(&mut self) {
        if let Some(mut buf) = self.st.buffer() {
            let current_text = buf.text();
            let current_styles = self.sbuf.text();
            
            let display_text = format!("{}{}", current_text, self.temp_s);
            let display_styles = format!("{}{}", current_styles, self.temp_b);
            
            buf.set_text(&display_text);
            self.sbuf.set_text(&display_styles);
            
            // Force the display to show all content by ensuring proper scrolling
            self.st.set_insert_position(buf.length());
            self.st.scroll(buf.count_lines(0, buf.length()), 0);
            
            self.temp_s.clear();
            self.temp_b.clear();
        }
    }
}

pub fn menu_cb(m: &mut impl MenuExt) {
    if let Some(term) = app::widget_from_id::<text::TextDisplay>("term") {
        if let Ok(mpath) = m.item_pathname(None) {
            match mpath.as_str() {
                "Copy\t" => {
                    if let Some(buffer) = term.buffer() {
                        app::copy2(&buffer.selection_text());
                    }
                },
                "Paste\t" => app::paste_text2(&term),
                _ => (),
            }
        }
    }
}

pub fn init_menu(m: &mut (impl MenuExt + 'static)) {
    m.add(
        "Copy\t",
        Shortcut::Ctrl | Key::Insert,
        menu::MenuFlag::Normal,
        menu_cb,
    );
    m.add(
        "Paste\t",
        Shortcut::Shift | Key::Insert,
        menu::MenuFlag::Normal,
        menu_cb,
    );
}

pub struct PPTerm {
    g: group::Group,
    st: text::TextDisplay,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    thread_handle: Option<JoinHandle<()>>,
    master_pty: Option<Arc<Mutex<Box<dyn MasterPty + Send>>>>,
    shutdown_flag: Arc<AtomicBool>,
    cols: u16,
    rows: u16,
}

impl Default for PPTerm {
    fn default() -> Self {
        PPTerm::new(0, 0, 0, 0, None)
    }
}

impl PPTerm {
    pub fn new<L: Into<Option<&'static str>>>(x: i32, y: i32, w: i32, h: i32, label: L) -> Self {
        Self::with_dimensions_and_scrollback(x, y, w, h, label, 80, 24, 1000)
    }

    pub fn with_dimensions<L: Into<Option<&'static str>>>(x: i32, y: i32, w: i32, h: i32, label: L, cols: u16, rows: u16) -> Self {
        Self::with_dimensions_and_scrollback(x, y, w, h, label, cols, rows, 1000)
    }

    pub fn with_dimensions_and_scrollback<L: Into<Option<&'static str>>>(x: i32, y: i32, w: i32, h: i32, label: L, cols: u16, rows: u16, scrollback_lines: usize) -> Self {
        let mut g = group::Group::new(x, y, w, h, label).with_id("term_group");
        let mut st = text::TextDisplay::new(x, y, w, h, None).with_id("term");
        let mut m = menu::MenuButton::default()
            .with_type(menu::MenuButtonType::Popup3)
            .with_id("pop2");
        init_menu(&mut m);
        g.end();
        st.set_cursor_color(Color::White);
        st.show_cursor(true);
        st.set_color(Color::Black);
        st.set_cursor_style(text::Cursor::Block);
        st.wrap_mode(text::WrapMode::AtBounds, 0);
        let buf = text::TextBuffer::default();
        st.set_buffer(buf);
        let styles = styles::init();
        let sbuf = text::TextBuffer::default();
        st.set_highlight_data(sbuf.clone(), styles);

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let master_pty_for_resize = Arc::new(Mutex::new(None::<Arc<Mutex<Box<dyn MasterPty + Send>>>>));
        
        g.resize_callback({
            let mut st = st.clone();
            let master_pty_clone = master_pty_for_resize.clone();
            let cols = cols;
            let rows = rows;
            move |_, x, y, w, h| {
                m.resize(x, y, w, h);
                st.resize(x, y, w, h);
                
                // Calculate new terminal dimensions based on widget size
                let char_width = 8;
                let char_height = 16;
                let new_cols = ((w as f32 / char_width as f32).floor() as u16).max(10);
                let new_rows = ((h as f32 / char_height as f32).floor() as u16).max(3);
                
                // Resize PTY if available
                if let Ok(pty_option) = master_pty_clone.lock() {
                    if let Some(ref pty) = *pty_option {
                        let _ = pty::resize_pty(pty, new_cols, new_rows);
                    }
                }
            }
        });

        let scrollback = Arc::new(Mutex::new(ScrollbackBuffer::new(scrollback_lines)));
        let performer = VteParser::new(st.clone(), sbuf.clone(), scrollback.clone(), rows);
        let (writer, thread_handle, master_pty) = pty::start(performer, cols, rows, shutdown_flag.clone());

        // Store master_pty for resize callback
        if let Some(ref pty) = master_pty {
            if let Ok(mut pty_option) = master_pty_for_resize.lock() {
                *pty_option = Some(pty.clone());
            }
        }

        if let Some(ref writer_ref) = writer {
            st.handle({
                let writer = writer_ref.clone();
                move |t, ev| match ev {
                    Event::KeyDown => {
                        let key = app::event_key();
                        match key {
                            #[cfg(windows)]
                            Key::BackSpace => {
                                if let Ok(mut w) = writer.lock() {
                                    let _ = w.write_all(b"\x7f");
                                }
                            },
                            Key::Up => {
                                if let Ok(mut w) = writer.lock() {
                                    let _ = w.write_all(UP);
                                }
                            },
                            Key::Down => {
                                if let Ok(mut w) = writer.lock() {
                                    let _ = w.write_all(DOWN);
                                }
                            },
                            Key::Left => {
                                // ignore for now
                                // if let Ok(mut w) = writer.lock() {
                                //     let _ = w.write_all(b"\x1b[D");
                                // }
                            },
                            Key::Right => {
                                if let Ok(mut w) = writer.lock() {
                                    let _ = w.write_all(b"\x1b[C");
                                }
                            },
                            _ => {
                                if app::event_state() == EventState::Ctrl | EventState::Shift {
                                    if key == Key::from_char('v') {
                                        app::paste_text2(t);
                                    }
                                } else {
                                    let txt = app::event_text();
                                    if let Ok(mut w) = writer.lock() {
                                        let _ = w.write_all(txt.as_bytes());
                                    }
                                }
                            }
                        }
                        true
                    }
                    Event::Paste => {
                        let txt = app::event_text();
                        if let Ok(mut w) = writer.lock() {
                            let _ = w.write_all(txt.as_bytes());
                        }
                        true
                    }
                    _ => false,
                }
            });
        }

        Self { g, st, writer, thread_handle, master_pty, shutdown_flag, cols, rows }
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn resize_terminal(&mut self, cols: u16, rows: u16) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref pty) = self.master_pty {
            pty::resize_pty(pty, cols, rows)?;
            self.cols = cols;
            self.rows = rows;
            Ok(())
        } else {
            Err("No PTY available for resizing".into())
        }
    }

    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
    }

    pub fn write_all(&self, s: &[u8]) -> Result<(), io::Error> {
        if let Some(writer) = &self.writer {
            match writer.lock() {
                Ok(mut w) => w.write_all(s),
                Err(_) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to acquire writer lock",
                ))
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "No writer available",
            ))
        }
    }
}

impl Drop for PPTerm {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(handle) = self.thread_handle.take() {
            // Use a timeout to avoid hanging on close
            std::thread::spawn(move || {
                let _ = handle.join();
            });
        }
    }
}

fltk::widget_extends!(PPTerm, group::Group, g);
