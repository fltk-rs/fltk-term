use fltk::{prelude::*, *};
use fltk_term::PPTerm;

fn main() {
    let a = app::App::default();
    let mut w = window::Window::default().with_size(600, 400);
    let term = PPTerm::default().size_of_parent();
    w.end();
    w.show();

    // app::add_timeout3(0.2, move |_| {
    // Test the original command that should show red text
    term.write_all(r#"echo -e "\033[1;31mHELLO""#.as_bytes())
        .unwrap();
    term.write_all(b"\n").unwrap();
    // });

    a.run().unwrap();
}
