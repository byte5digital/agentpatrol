//! The popover as the shell sets it (spec section 8.1): every show makes
//! AgentSweep the active app and focuses the popover, so a click outside
//! takes the focus away and hides it.
//!
//! The suite runs no window server and no webview, so this reads the files
//! as they ship. The click outside and the press are a hand check (section
//! 19 of `docs/hand-checks.md`).

use std::path::Path;

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{}", path.display()))
}

/// The text of the function `name` in `source`, up to the next top-level
/// item.
fn function<'a>(source: &'a str, name: &str) -> &'a str {
    let start = source
        .find(&format!("fn {name}("))
        .or_else(|| source.find(&format!("function {name}(")))
        .unwrap_or_else(|| panic!("no function {name}"));
    let rest = &source[start..];
    let end = rest.find("\n}\n").map_or(rest.len(), |at| at + 3);
    &rest[..end]
}

/// The position of `needle` in `text`.
fn at(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("no {needle} in:\n{text}"))
}

/// A menu bar app is not made active by a click on its icon, and a window
/// of an app that is not active never loses the focus. So the popover
/// activates the app before it shows, focuses itself after, and hides when
/// the focus goes.
#[test]
fn every_show_activates_the_app_and_focuses_the_popover() {
    let app = read("src/app.rs");
    let toggle = function(&app, "toggle");
    let activated = at(toggle, "activate(app)");
    let shown = at(toggle, "window.show()");
    let focused = at(toggle, "window.set_focus()");
    assert!(activated < shown && shown < focused, "{toggle}");
    let activate = function(&app, "activate");
    assert!(activate.contains(".activate()"), "{activate}");
    assert!(app.contains(
        "if let WindowEvent::Focused(false) = event {\n                        let _ = hide.hide();"
    ));
}
