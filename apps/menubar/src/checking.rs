//! "Checking your setup…": a running Full Scan on the Overview and in the
//! popover (spec sections 8.1 and 8.3).
//!
//! The Overview shows a title, a bar and one short line naming what is being
//! checked: the Agent while the Scan enumerates, then the project or the
//! Agent of the Items in hand. The popover shows "Checking…" with the same
//! bar. Both follow the daemon's `scan-progress` events, which only a Full
//! Scan sends, so a Change Scan shows nothing of this. The block goes when
//! the Scan ends.
//!
//! The Simple Posture names a project by its folder, never its path; the
//! Developer Posture says the path and the glossary's words.
//!
//! From a click on **Turn on protection** until the daemon answers, which
//! can take minutes, the Overview and the popover show "Starting
//! protection…", "This can take a few minutes." and a bar that does not know
//! how far ([`starting`]); the Full Scan's bar follows once the daemon
//! answers.

use std::path::Path;

use agentsweep_core::settings::Posture;
use agentsweep_ipc::protocol::{InHand, ScanPhase};
use serde_json::{Value, json};

use crate::findings::ScanLine;

/// The Overview's title while a Full Scan runs.
pub const TITLE: &str = "Checking your setup…";
/// The popover's word for the same.
pub const POPOVER_TITLE: &str = "Checking…";
/// The line under "Starting protection…" until the daemon answers.
pub const TAKES_TIME: &str = "This can take a few minutes.";
/// The line while the Scan has nothing in hand to name.
pub const LOOKING: &str = "Looking for what to check.";

/// The workspace adapter's id: its Items are the projects' own git files.
const WORKSPACE: &str = "workspace";

/// How far the bar is: `done` of `total`, and a bar that does not know how
/// far yet while `total` is zero.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Bar {
    /// The Items done.
    pub done: usize,
    /// The Items there are, zero while the Scan still enumerates.
    pub total: usize,
}

impl Bar {
    fn to_value(self) -> Value {
        json!({ "done": self.done, "total": self.total })
    }
}

/// The Overview's block while a Full Scan runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checking {
    /// "Checking your setup…".
    pub title: &'static str,
    /// The bar.
    pub bar: Bar,
    /// "Checked 12 of 43.", empty while the Scan enumerates.
    pub count: String,
    /// What is being checked, empty when there is nothing to name.
    pub line: String,
}

impl Checking {
    /// The value the window receives under `overview.checking`.
    pub fn to_value(&self) -> Value {
        json!({
            "title": self.title,
            "bar": self.bar.to_value(),
            "count": self.count,
            "line": self.line,
        })
    }
}

/// The block for the Scan in hand, or none when no Full Scan runs.
pub fn of_scan(scan: &ScanLine, posture: Posture) -> Option<Checking> {
    if !(scan.running && scan.full) {
        return None;
    }
    Some(Checking {
        title: TITLE,
        bar: bar_of(scan),
        count: count(scan, posture),
        line: line(scan.phase, scan.in_hand.as_ref(), posture),
    })
}

/// The block before the daemon told anything about the first check: a bar
/// that does not know its total, and `line`.
pub fn waiting(line: &str) -> Checking {
    Checking {
        title: TITLE,
        bar: Bar::default(),
        count: String::new(),
        line: line.to_string(),
    }
}

/// The block from a click on Turn on protection until the daemon answers.
pub fn starting() -> Checking {
    Checking {
        title: crate::banner::STARTING_PROTECTION,
        ..waiting(TAKES_TIME)
    }
}

/// The popover's bar for the Scan in hand, or none when no Full Scan runs.
pub fn popover_bar(scan: &ScanLine) -> Option<Bar> {
    (scan.running && scan.full).then(|| bar_of(scan))
}

fn bar_of(scan: &ScanLine) -> Bar {
    match scan.phase {
        ScanPhase::Enumerating => Bar::default(),
        ScanPhase::Checking => Bar {
            done: scan.done.min(scan.total),
            total: scan.total,
        },
    }
}

/// The count under the bar: "Checked 12 of 43." in the Simple Posture, the
/// command line interface's "Checked 12 of 43 Items." in the Developer
/// Posture, nothing while the Scan enumerates.
fn count(scan: &ScanLine, posture: Posture) -> String {
    if scan.phase == ScanPhase::Enumerating || scan.total == 0 {
        return String::new();
    }
    match posture {
        Posture::Simple => format!("Checked {} of {}.", scan.done, scan.total),
        Posture::Developer => crate::findings::progress_line(scan.done, scan.total),
    }
}

/// The one line naming what is being checked.
pub fn line(phase: ScanPhase, in_hand: Option<&InHand>, posture: Posture) -> String {
    let Some(in_hand) = in_hand else {
        return match phase {
            ScanPhase::Enumerating => LOOKING.to_string(),
            ScanPhase::Checking => String::new(),
        };
    };
    let agent = match in_hand.display_name.is_empty() {
        true => in_hand.agent.as_str(),
        false => in_hand.display_name.as_str(),
    };
    let project = in_hand.project.as_deref();
    match (posture, phase) {
        (Posture::Simple, ScanPhase::Enumerating) if in_hand.agent == WORKSPACE => {
            "Looking at your projects.".to_string()
        }
        (Posture::Simple, ScanPhase::Enumerating) => format!("Looking at {agent}."),
        (Posture::Simple, ScanPhase::Checking) => match project {
            Some(root) => format!("Checking the project '{}'.", folder(root)),
            None => format!("Checking {agent}."),
        },
        (Posture::Developer, ScanPhase::Enumerating) => format!("Enumerating {agent}."),
        (Posture::Developer, ScanPhase::Checking) => match project {
            Some(root) => format!("Checking {root} ({agent})."),
            None => format!("Checking {agent}."),
        },
    }
}

/// A project as the Simple Posture names it: its folder's own name.
fn folder(root: &str) -> &str {
    Path::new(root)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "/tmp/home.invalid/src/shop";

    fn hand(agent: &str, name: &str, project: Option<&str>) -> InHand {
        InHand {
            agent: agent.to_string(),
            display_name: name.to_string(),
            project: project.map(str::to_string),
        }
    }

    fn scan(done: usize, total: usize, in_hand: Option<InHand>) -> ScanLine {
        ScanLine {
            running: true,
            done,
            total,
            full: true,
            phase: ScanPhase::of_total(total),
            in_hand,
            ..ScanLine::default()
        }
    }

    /// The line names the Agent while the Scan enumerates, then the project
    /// or the Agent in hand; the Simple Posture never shows a path.
    #[test]
    fn the_line_names_what_is_being_checked_in_each_posture() {
        let claude = hand("claude-code", "Claude Code", None);
        let workspace = hand("workspace", "Project files", None);
        let project = hand("codex", "Codex", Some(ROOT));
        let cases = [
            (ScanPhase::Enumerating, None, LOOKING, LOOKING),
            (
                ScanPhase::Enumerating,
                Some(&workspace),
                "Looking at your projects.",
                "Enumerating Project files.",
            ),
            (
                ScanPhase::Enumerating,
                Some(&claude),
                "Looking at Claude Code.",
                "Enumerating Claude Code.",
            ),
            (
                ScanPhase::Checking,
                Some(&claude),
                "Checking Claude Code.",
                "Checking Claude Code.",
            ),
            (
                ScanPhase::Checking,
                Some(&project),
                "Checking the project 'shop'.",
                "Checking /tmp/home.invalid/src/shop (Codex).",
            ),
            (ScanPhase::Checking, None, "", ""),
        ];
        for (phase, in_hand, simple, developer) in cases {
            assert_eq!(line(phase, in_hand, Posture::Simple), simple);
            assert_eq!(line(phase, in_hand, Posture::Developer), developer);
            assert!(!line(phase, in_hand, Posture::Simple).contains('/'));
        }
    }

    /// A Full Scan shows the title, the bar, the count and the line; the
    /// bar does not know its total while the Scan enumerates.
    #[test]
    fn a_full_scan_shows_its_bar_its_count_and_its_line() {
        let looking = of_scan(
            &scan(0, 0, Some(hand("claude-code", "Claude Code", None))),
            Posture::Simple,
        )
        .expect("a block");
        assert_eq!(looking.title, TITLE);
        assert_eq!(looking.bar, Bar { done: 0, total: 0 });
        assert_eq!(looking.count, "");
        assert_eq!(looking.line, "Looking at Claude Code.");

        let checking = scan(
            120,
            682,
            Some(hand("claude-code", "Claude Code", Some(ROOT))),
        );
        let simple = of_scan(&checking, Posture::Simple).expect("a block");
        assert_eq!(
            simple.bar,
            Bar {
                done: 120,
                total: 682
            }
        );
        assert_eq!(simple.count, "Checked 120 of 682.");
        assert_eq!(simple.line, "Checking the project 'shop'.");
        let developer = of_scan(&checking, Posture::Developer).expect("a block");
        assert_eq!(developer.count, "Checked 120 of 682 Items.");
        assert_eq!(
            simple.to_value(),
            json!({
                "title": TITLE,
                "bar": { "done": 120, "total": 682 },
                "count": "Checked 120 of 682.",
                "line": "Checking the project 'shop'.",
            })
        );
        assert_eq!(popover_bar(&checking), Some(simple.bar));
    }

    /// A Scan that never reported progress is a Change Scan, and one that is
    /// over shows nothing: no block and no popover bar.
    #[test]
    fn a_change_scan_and_an_idle_daemon_show_no_bar() {
        let change = ScanLine {
            running: true,
            ..ScanLine::default()
        };
        assert_eq!(of_scan(&change, Posture::Simple), None);
        assert_eq!(popover_bar(&change), None);
        let over = ScanLine {
            running: false,
            ..scan(682, 682, None)
        };
        assert_eq!(of_scan(&over, Posture::Developer), None);
        assert_eq!(popover_bar(&over), None);
    }

    /// Until the daemon answers, starting protection shows its title, the
    /// line that it takes time, and a bar that does not know how far.
    #[test]
    fn starting_protection_says_it_takes_time() {
        let block = starting();
        assert_eq!(block.title, "Starting protection…");
        assert_eq!(block.line, "This can take a few minutes.");
        assert_eq!(block.bar, Bar::default());
        assert_eq!(block.count, "");
    }

    /// The Overview draws the block from the frame's value: the title, the
    /// bar through the module the popover uses, the count and the line.
    #[test]
    fn the_overview_draws_the_block_and_its_bar() {
        let page = include_str!("../ui/window.html");
        assert!(page.contains(r#"<div class="checking" id="overview-checking" hidden>"#));
        assert!(page.contains(r#"<progress id="checking-bar""#));
        let script = include_str!("../ui/window.js");
        assert!(script.contains(r#"import { renderBar } from "./checking.js";"#));
        assert!(script.contains("renderChecking(overview.checking ?? null);"));
        assert!(
            script.contains(
                r#"renderBar($("checking-bar"), checking.bar.done, checking.bar.total);"#
            )
        );
        let module = include_str!("../ui/checking.js");
        assert!(module.contains("export function renderBar(bar, done, total)"));
        assert!(module.contains(r#"bar.removeAttribute("value");"#));
        assert!(!module.contains("innerHTML"));
    }

    /// Every text is short and plain, and none carries an em-dash.
    #[test]
    fn the_texts_are_short_and_carry_no_em_dash() {
        for text in [TITLE, POPOVER_TITLE, TAKES_TIME, LOOKING] {
            assert!(text.len() <= 30, "{text}");
            assert!(!text.contains('\u{2014}'), "{text}");
        }
    }
}
