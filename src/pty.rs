use crate::cell_performer::CellsPerformer;
use crate::cells::CellBuffer;
use fltk::app;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::env;
use std::io::{Read, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use vte::Parser;

pub(crate) struct PtyHandles {
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub thread_handle: JoinHandle<()>,
    pub master_pty: Arc<Mutex<Box<dyn MasterPty + Send>>>,
}

pub(crate) fn start(
    buffer: Arc<Mutex<CellBuffer>>,
    cols: u16,
    rows: u16,
    shutdown_flag: Arc<AtomicBool>,
) -> Option<PtyHandles> {
    let pair = match native_pty_system().openpty(PtySize {
        cols,
        rows,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(_) => return None,
    };

    let mut cmd = if cfg!(target_os = "windows") {
        CommandBuilder::new("cmd.exe")
    } else {
        CommandBuilder::new("/bin/bash")
    };
    if let Ok(cwd) = env::current_dir() {
        cmd.cwd(cwd);
    }
    if let Ok(path) = env::var("PATH") {
        cmd.env("PATH", path);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(_) => return None,
    };
    let mut reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(_) => return None,
    };
    let writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(_) => return None,
    };
    let master_pty = Arc::new(Mutex::new(pair.master));
    let writer = Arc::new(Mutex::new(writer));
    std::mem::forget(pair.slave);

    let mut statemachine = Parser::new();
    
    app::sleep(0.1);

    let thread_handle = thread::spawn({
        move || {
            #[cfg(feature = "debug-term")]
            eprintln!("PTY thread (cells) started");

            while !shutdown_flag.load(Ordering::Relaxed) {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {}
                    Err(_) => break,
                }

                let mut msg = [0u8; 4096];
                match reader.read(&mut msg) {
                    Ok(0) => break,
                    Ok(sz) => {
                        let msg = &msg[0..sz];
                        if let Ok(mut buf) = buffer.lock() {
                            let mut perf = CellsPerformer::new(&mut buf);
                            for byte in msg {
                                statemachine.advance(&mut perf, *byte);
                            }
                        }
                        app::awake();
                    }
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            app::sleep(0.01);
                        }
                        std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::BrokenPipe => {
                            break;
                        }
                        _ => {
                            #[cfg(feature = "debug-term")]
                            eprintln!("PTY read error: {}", e);
                            app::sleep(0.01);
                        }
                    },
                }
                app::sleep(0.03);
            }
        }
    });

    Some(PtyHandles {
        writer,
        thread_handle,
        master_pty,
    })
}

/// Resize the PTY to new dimensions
pub(crate) fn resize_pty(
    master_pty: &Arc<Mutex<Box<dyn MasterPty + Send>>>,
    cols: u16,
    rows: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(pty) = master_pty.lock() {
        pty.resize(PtySize {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    } else {
        Err("Failed to acquire PTY lock".into())
    }
}
