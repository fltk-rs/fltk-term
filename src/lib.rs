use fltk::{enums::*, prelude::*, *};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{
    env,
    io::{self, Read, Write},
    str,
    sync::{Arc, Mutex},
    thread,
};

pub fn menu_cb(m: &mut impl MenuExt) {
    let term: text::TextDisplay = app::widget_from_id("term").unwrap();
    if let Ok(mpath) = m.item_pathname(None) {
        match mpath.as_str() {
            "Copy\t" => app::copy2(&term.buffer().unwrap().selection_text()),
            "Paste\t" => app::paste_text2(&term),
            _ => (),
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
    // st: group::experimental::Terminal,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}
impl Default for PPTerm {
    fn default() -> Self {
        PPTerm::new(0, 0, 0, 0, None)
    }
}

impl PPTerm {
    pub fn new<L: Into<Option<&'static str>>>(x: i32, y: i32, w: i32, h: i32, label: L) -> Self {
        let mut g = group::Group::new(x, y, w, h, label).with_id("term_group");
        let mut st = group::experimental::Terminal::default().with_id("term");
        let mut m = menu::MenuButton::default()
            .with_type(menu::MenuButtonType::Popup3)
            .with_id("pop2");
        init_menu(&mut m);
        g.end();
        g.resize_callback({
            let mut st = st.clone();
            move |_, x, y, w, h| {
                m.resize(x, y, w, h);
                st.resize(x, y, w, h);
            }
        });
        // Terminal handles many common ansi escape sequence
        st.set_ansi(true);
        let pair = native_pty_system()
            .openpty(PtySize {
                cols: 120,
                rows: 16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();

        let mut cmd = if cfg!(target_os = "windows") {
            env::set_var("TERM", "xterm-mono");
            CommandBuilder::new("cmd.exe")
        } else {
            env::set_var("TERM", "vt100");
            CommandBuilder::new("/bin/bash")
        };
        cmd.cwd(env::current_dir().unwrap());
        cmd.env("PATH", env::var("PATH").unwrap());

        let mut child = pair.slave.spawn_command(cmd).unwrap();
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let writer = Arc::new(Mutex::new(writer));
        std::mem::forget(pair);

        #[cfg(not(windows))]
        {
            thread::spawn({
                let mut st = st.clone();
                move || {
                    let mut s = Vec::new();
                    while child.try_wait().is_ok() {
                        let mut msg = [0u8; 2000];
                        if let Ok(sz) = reader.read(&mut msg) {
                            let msg = &msg[0..sz];
                            s.extend_from_slice(&msg[0..sz]);
                            match str::from_utf8(&s) {
                                Ok(text) => {
                                    if text != "\x07" {
                                        st.append(text);
                                    }
                                    s.clear();
                                }
                                Err(z) => {
                                    let z = z.valid_up_to();
                                    st.append_u8(&msg[0..z]);
                                    s.extend_from_slice(&msg[z..]);
                                }
                            }
                            app::awake();
                        }
                        app::sleep(0.03);
                    }
                }
            });
        }

        #[cfg(windows)]
        {
            // windows quirk
            app::sleep(0.05);
            thread::spawn({
                let mut st = st.clone();
                move || {
                    while child.try_wait().is_ok() {
                        let mut msg = [0u8; 1024];
                        if let Ok(sz) = reader.read(&mut msg) {
                            let msg = &msg[0..sz];
                            st.append2(msg);
                        }
                        app::sleep(0.03);
                    }
                }
            });
        }

        st.handle({
            let writer = writer.clone();
            move |t, ev| match ev {
                Event::Focus => true,
                Event::KeyDown => {
                    let key = app::event_key();
                    if key == Key::Up {
                        writer.lock().unwrap().write_all(b"\x10").unwrap();
                        // t.scroll(t.count_lines(0, t.buffer().unwrap().length(), true), 0);
                    } else if key == Key::Down {
                        writer.lock().unwrap().write_all(b"\x0E").unwrap();
                    } else if key == Key::from_char('v') && app::event_state() == EventState::Ctrl {
                        app::paste(t);
                    } else {
                        let txt = app::event_text();
                        writer.lock().unwrap().write_all(txt.as_bytes()).unwrap();
                    }
                    true
                }
                Event::KeyUp => {
                    if app::event_key() == Key::Up {
                        // t.scroll(t.count_lines(0, t.buffer().unwrap().length(), true), 0);
                        true
                    } else {
                        false
                    }
                }
                Event::Paste => {
                    let txt = app::event_text();
                    writer.lock().unwrap().write_all(txt.as_bytes()).unwrap();
                    true
                }
                _ => false,
            }
        });

        Self { g, writer }
    }

    pub fn write_all(&self, s: &[u8]) -> Result<(), io::Error> {
        self.writer.lock().unwrap().write_all(s)
    }
}

fltk::widget_extends!(PPTerm, group::Group, g);
