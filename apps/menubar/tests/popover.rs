//! The popover as the shell and the banner script set it (spec sections 8.1
//! and 8.6): every show makes AgentSweep the active app and focuses the
//! popover, so a click outside takes the focus away and hides it; and
//! **Turn on protection** gives way to "Starting protection…" from the click
//! until it fails.
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

/// A press of a button that gives way hides the banner's buttons and says
/// its working line before it is asked; they come back only when it fails,
/// stay hidden through every render meanwhile, also when the banner turns to
/// another one, and are offered again once the banner went away.
#[test]
fn turn_on_protection_gives_way_to_its_line_until_it_fails() {
    let banner = read("ui/banner.js");
    let render = function(&banner, "renderBanner");
    let gave_way = at(render, "givenWay.add(root)");
    let said = at(render, "say(button.working)");
    let asked = at(render, "await invoke(button.command");
    let failed = at(render, "catch (why)");
    let back = at(
        render,
        "givenWay.delete(root);\n          buttons.hidden = false;",
    );
    assert!(gave_way < said && said < asked && asked < failed && failed < back);
    assert!(render[gave_way..said].contains("buttons.hidden = true"));
    assert!(render[..gave_way].contains("if (button.givesWay)"));
    // Every render keeps them hidden while it stands, after `keep`.
    let kept = at(render, "keep(buttons,");
    let hidden = render
        .rfind("buttons.hidden = givenWay.has(root)")
        .expect("every render");
    assert!(kept < hidden, "{render}");
    // A banner that went away offers its buttons again when it comes back.
    let gone = at(render, "if (banner === null) {");
    assert!(render[gone..].starts_with("if (banner === null) {\n    givenWay.delete(root);"));
    // No button greys out to give way: the others still wait as they did.
    assert!(render.contains("node.disabled = true"));
}
