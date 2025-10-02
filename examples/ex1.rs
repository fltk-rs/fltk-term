use fltk::{prelude::*, *};
use fltk_term::PPTerm;

fn main() {
    let a = app::App::default();
    let mut w = window::Window::default().with_size(600, 400);
    // Defer PTY start until after window sizing
    let mut term = PPTerm::new_deferred(0, 0, 0, 0, None).size_of_parent();
    w.end();
    w.show();

    // Start the PTY with correct initial cols/rows
    term.start();

    // Test the original command that should show red text
    term
        .write_all(b"echo -e \"\x1b[1;31mHELLO\"\n")
        .unwrap();

    a.run().unwrap();
}
