//! The all-clear sentence names the last check in local time in the popover
//! and on the Overview, never as the daemon's UTC timestamp; the command
//! line interface keeps its own form (spec section 8.1).
//!
//! The suite runs no webview, so this reads the scripts as they ship. The
//! rendered sentence is a hand check (section 2 of `docs/hand-checks.md`).

use std::path::Path;

fn ui(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{}", path.display()))
}

/// One module turns every timestamp in a sentence into local time, and
/// writes a moment the way the popover does.
#[test]
fn one_module_writes_a_sentence_in_local_time() {
    let module = ui("when.js");
    assert!(module.contains("export function when(text)"), "{module}");
    assert!(
        module.contains("export function inLocalTime(sentence)"),
        "{module}"
    );
    assert!(module.contains("dateStyle: \"medium\", timeStyle: \"short\""));
    // The form the daemon writes: 2026-11-10T08:12:44.000Z.
    assert!(
        module.contains(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}"),
        "{module}"
    );
    assert!(!module.contains("innerHTML"));
}

/// The popover and the Overview set their sentence through it.
#[test]
fn the_popover_and_the_overview_say_the_time_in_local_time() {
    let popover = ui("popover.js");
    assert!(popover.contains("import { inLocalTime } from \"./when.js\";"));
    assert!(popover.contains("sentence.textContent = inLocalTime(view.sentence ?? \"\");"));
    let window = ui("window.js");
    assert!(window.contains("from \"./when.js\";"));
    assert!(window.contains(
        "$(\"overview-sentence\").textContent = inLocalTime(overview.sentence ?? \"\");"
    ));
    assert!(
        !window.contains("function when("),
        "the window writes a moment through the module"
    );
}
