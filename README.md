# fltk-term

The fltk-term terminal is quite minimal, supports a limited subset of ansi escape sequences using [vte](https://github.com/alacritty/vte), i.e. don't expect to run vim in it!, and is powered by [portable-pty](https://github.com/wez/wezterm/pty). 

## Known issues
- On Windows, the terminal defaults to cmd. More ansi escape sequences need to be handled to support powershell. 

## Usage

```rust
use fltk_term::PPTerm;
use fltk::{prelude::*, *};

fn main() {
    let a = app::App::default();
    let mut w = window::Window::default().with_size(600, 400);
    // Defer PTY start until after window sizing to avoid truncated first prompt
    let mut term = PPTerm::new_deferred(0, 0, 0, 0, None).size_of_parent();
    w.end();
    w.show();

    // Start the PTY now that the widget has a real size
    term.start();

    app::add_timeout3(0.2, move |_| {
        term.write_all(b"echo -e \"\x1b[1;31mHELLO\"\n").unwrap();
    });

    a.run().unwrap();
}
```
