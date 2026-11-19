//! The AgentPatrol menu bar app. Everything is in the library beside this file.
//!
//! The app takes one argument, `--windowless`, with which the daemon starts
//! it into the menu bar with no window when it has something to tell (spec
//! section 6.7), and `--finish-uninstall`, which `agentpatrol uninstall` runs
//! once the daemon has applied the user's choice: it quits a running
//! AgentPatrol and unregisters the daemon and the login item, says on stderr
//! what it could not do, and exits (spec section 9.6). Beside them are two
//! developer flags:
//! `--developer-status` prints where the daemon's and the login item's
//! registrations stand, and `--developer-unregister` unregisters both for the
//! bundle it is run from. Each exits without opening anything. It is how
//! a hand check takes a test install off the machine (`docs/hand-checks.md`)
//! without the choice about Quarantine that Uninstall in Settings asks for.

use std::process::ExitCode;

use agentpatrol_menubar::supervision::{self, SystemServices};
use agentpatrol_menubar::uninstall::{self, SystemRunning};
use agentpatrol_menubar::window::Start;

fn main() -> ExitCode {
    if std::env::args()
        .skip(1)
        .any(|argument| argument == "--finish-uninstall")
    {
        let left = uninstall::finish(&SystemRunning, &SystemServices);
        for line in &left {
            eprintln!("{line}");
        }
        return match left.is_empty() {
            true => ExitCode::SUCCESS,
            false => ExitCode::FAILURE,
        };
    }
    if std::env::args()
        .skip(1)
        .any(|argument| argument == "--developer-status")
    {
        use agentpatrol_menubar::supervision::Services;
        println!("daemon: {}", SystemServices.agent());
        println!("login item: {}", SystemServices.login_item());
        return ExitCode::SUCCESS;
    }
    if std::env::args()
        .skip(1)
        .any(|argument| argument == "--developer-unregister")
    {
        let failures = supervision::unregister_everything(&SystemServices);
        for failure in &failures {
            eprintln!("agentpatrol-menubar: {failure}");
        }
        return match failures.is_empty() {
            true => {
                println!("agentpatrol-menubar: the daemon and the login item are unregistered");
                ExitCode::SUCCESS
            }
            false => ExitCode::FAILURE,
        };
    }
    agentpatrol_menubar::app::run(Start::of(std::env::args().skip(1)));
    ExitCode::SUCCESS
}
