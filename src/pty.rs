use fltk::app;
use portable_pty::{native_pty_system, CommandBuilder, PtySize, MasterPty};
use std::env;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};
use vte::Parser;

pub(crate) fn start(
    mut performer: crate::VteParser,
    cols: u16,
    rows: u16,
    shutdown_flag: Arc<AtomicBool>,
) -> (Option<Arc<Mutex<Box<dyn Write + Send>>>>, Option<JoinHandle<()>>, Option<Arc<Mutex<Box<dyn MasterPty + Send>>>>) {
    let pair = match native_pty_system().openpty(PtySize {
        cols,
        rows,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(_) => return (None, None, None),
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
    
    // Set environment variables to ensure ANSI colors work  
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(_) => return (None, None, None),
    };
    let mut reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(_) => return (None, None, None),
    };
    let writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(_) => return (None, None, None),
    };
    
    let master_pty = Arc::new(Mutex::new(pair.master));
    let writer = Arc::new(Mutex::new(writer));
    // Note: pair.slave was consumed by spawn_command, only pair.master remains
    // and we've moved it to master_pty, so we don't need to forget the pair

    let mut statemachine = Parser::new();

    #[cfg(windows)]
    app::sleep(0.05);

    let thread_handle = thread::spawn({
        move || {
            #[cfg(feature = "debug-term")]
            eprintln!("PTY thread started - about to start reading");
            
            while !shutdown_flag.load(Ordering::Relaxed) {
                // Check if child process is still alive
                match child.try_wait() {
                    Ok(Some(_exit_status)) => {
                        // Child has exited
                        break;
                    }
                    Ok(None) => {
                        // Child is still running, continue reading
                    }
                    Err(_) => {
                        // Error checking child status
                        break;
                    }
                }

                let mut msg = [0u8; 4096];
                match reader.read(&mut msg) {
                    Ok(0) => {
                        // EOF reached
                        break;
                    }
                    Ok(sz) => {
                        let msg = &msg[0..sz];
                        #[cfg(feature = "debug-term")]
                        {
                            let text_repr = String::from_utf8_lossy(msg);
                            eprintln!("PTY read {} bytes: {:?}", sz, text_repr);
                        }
                        for byte in msg {
                            statemachine.advance(&mut performer, *byte);
                        }
                        performer.myprint();
                        app::awake();
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            // No data available right now, just continue the loop
                            // This allows us to check the shutdown flag
                        } else {
                            // Real error, sleep briefly and continue
                            app::sleep(0.01);
                        }
                    }
                }
                app::sleep(0.03);
            }
        }
    });
    (Some(writer), Some(thread_handle), Some(master_pty))
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
