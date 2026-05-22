//! Quick harness to render an app icon PNG straight from a bundle id, so we can
//! verify macOS 26 `.icon` rendering without launching the full Tauri app.
//!
//! It calls the *real* `app_icons::render_icon_png`, so whatever you see here is
//! exactly what the app would put in the recordings list.
//!
//! Usage:
//!   cargo run -p nbp --example render_icon
//!   cargo run -p nbp --example render_icon -- com.tinyspeck.slackmacgap com.apple.Safari
//!
//! With no args it renders a default mix of a macOS 26 dynamic `.icon` app
//! (Slack) and a classic `.icns` app (Safari). Each PNG is written to the temp
//! dir and opened in Preview for an eyeball check.

use objc2::runtime::{AnyClass, AnyObject};

fn main() {
    // AppKit drawing (lockFocus/drawInRect inside render_icon_png) needs the
    // graphics environment up; in a plain binary that means creating the shared
    // NSApplication on the main thread, which `main` already is.
    unsafe {
        if let Some(cls) = AnyClass::get(c"NSApplication") {
            let _app: *mut AnyObject = objc2::msg_send![cls, sharedApplication];
        }
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let bundles = if args.is_empty() {
        vec![
            "com.tinyspeck.slackmacgap".to_string(), // macOS 26 dynamic .icon
            "com.apple.Safari".to_string(),          // classic .icns
        ]
    } else {
        args
    };

    let mut any_ok = false;
    for bid in &bundles {
        match nbp_lib::app_icons::render_icon_png(bid) {
            Some(png) => {
                let safe: String = bid
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
                    .collect();
                let path = std::env::temp_dir().join(format!("nbp-icon-{safe}.png"));
                match std::fs::write(&path, &png) {
                    Ok(()) => {
                        println!("OK   {bid}: {} bytes -> {}", png.len(), path.display());
                        let _ = std::process::Command::new("open").arg(&path).status();
                        any_ok = true;
                    }
                    Err(e) => println!("WRITE-FAIL {bid}: {e}"),
                }
            }
            None => println!("FAIL {bid}: render_icon_png returned None"),
        }
    }

    if !any_ok {
        std::process::exit(1);
    }
}
