//! A drag to the Trash, as the daemon takes it (spec sections 9.5 and 9.6).
//!
//! A bundle in a Trash folder is gone too: launchd starts the daemon from
//! there again at the next login, and that daemon takes the Gate out and
//! exits 0 before it opens a store, binds a socket or scans, and no daemon
//! ever registers the Gate under a command line interface in the Trash.
//!
//! The bundle watch has already judged the bundle and waited out its grace;
//! these tests start where it hands over. A bundle still away at the moment
//! of exit is gone: the daemon takes the Gate's entries and the Codex pins
//! out of both Agents, records each in the Event Log, and exits 0. A bundle
//! back at its path by then was replaced: the entries stay and it exits 75.
//!
//! Every daemon here runs against a temporary home with both Agents
//! installed, a temporary data directory, a temporary socket and the chain
//! key held in the process. The bundle is a path in the temporary directory,
//! and the command line interface the entries name is a path nothing runs.

#[path = "support/started.rs"]
mod started;
use started::Started;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use agentsweep_core::adapter::SurfaceAdapter;
use agentsweep_core::eventlog::{Action, EventLog};
use agentsweep_core::gate_registration::{AtomicWriter, GateCheck, GateCommand};
use agentsweep_core::keychain::{ChainKey, KeyError, KeySource, MemoryKeySource};
use agentsweep_core::paths::DataDirectory;
use agentsweep_core::state::Trigger;
use agentsweep_daemon::bundle_watch::Change;
use agentsweep_daemon::machine::Machine;
use agentsweep_daemon::options::{KeySourceChoice, LogChoice, Options};
use agentsweep_daemon::{Daemon, Tuning};
use agentsweep_ipc::peer::SameUser;

use agentsweep_adapter_claude_code::ClaudeCode;
use agentsweep_adapter_codex::Codex;

static NEXT: AtomicU32 = AtomicU32::new(0);

/// Where the bundle puts the command line interface. Nothing runs it.
const CLI: &str = "/Applications/AgentSweep.app/Contents/MacOS/agentsweep";

/// Another tool's handler, which the test puts into a group AgentSweep's
/// handler sits in.
const FOREIGN: &str = "bash ~/.claude/other-tool.sh";

/// The user's own Claude Code settings, with a hook of their own.
const USERS_SETTINGS: &str = r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash ~/.claude/lint.sh",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
"#;

/// The user's own Codex hooks file, with a group of their own.
const USERS_HOOKS: &str = r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash ~/.codex/lint.sh",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
"#;

/// The user's own Codex config layer.
const USERS_CONFIG: &str = "model = \"gpt-5-codex\"\n\n[features]\nhooks = true\n";

/// The one chain key the daemon and the test share.
#[derive(Clone, Debug)]
struct SharedKey(Arc<MemoryKeySource>);

impl KeySource for SharedKey {
    fn load(&self) -> Result<Option<ChainKey>, KeyError> {
        self.0.load()
    }

    fn renew(&self) -> Result<ChainKey, KeyError> {
        self.0.renew()
    }

    fn forget(&self) -> Result<(), KeyError> {
        self.0.forget()
    }
}

struct Sandbox {
    root: PathBuf,
    socket: PathBuf,
    keys: SharedKey,
    /// The daemons it started, which it waits for before its folder goes.
    started: Started,
}

impl Sandbox {
    fn new(purpose: &str) -> Sandbox {
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "agentsweep-bundle-gone-{purpose}-{}-{number}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("home/.claude")).expect("an installed Claude Code");
        std::fs::create_dir_all(root.join("home/.codex")).expect("an installed Codex");
        std::fs::create_dir_all(root.join("data")).expect("a data directory");
        std::fs::create_dir_all(root.join("Applications")).expect("an Applications folder");
        let sandbox = Sandbox {
            started: Started::default(),
            socket: std::env::temp_dir().join(format!("asb-{}-{number}.sock", std::process::id())),
            root,
            keys: SharedKey(Arc::new(MemoryKeySource::empty())),
        };
        std::fs::write(sandbox.claude_settings(), USERS_SETTINGS).expect("the user's settings");
        std::fs::write(sandbox.codex_hooks(), USERS_HOOKS).expect("the user's hooks");
        std::fs::write(sandbox.codex_config(), USERS_CONFIG).expect("the user's config");
        sandbox
    }

    fn data(&self) -> DataDirectory {
        DataDirectory::at(self.root.join("data"))
    }

    fn claude_settings(&self) -> PathBuf {
        self.root.join("home/.claude/settings.json")
    }

    fn codex_hooks(&self) -> PathBuf {
        self.root.join("home/.codex/hooks.json")
    }

    fn codex_config(&self) -> PathBuf {
        self.root.join("home/.codex/config.toml")
    }

    /// Where the bundle was. Nothing is there unless a test puts it there.
    fn bundle(&self) -> PathBuf {
        self.root.join("Applications/AgentSweep.app")
    }

    /// A bundle the Finder put into the sandbox home's Trash, renamed the way
    /// it renames one whose name is taken there. Nothing is in it.
    fn trashed_bundle(&self) -> PathBuf {
        self.root.join("home/.Trash/AgentSweep 12.50.04.app")
    }

    /// The command line interface inside the trashed bundle.
    fn trashed_cli(&self) -> GateCommand {
        GateCommand::at(self.trashed_bundle().join("Contents/MacOS/agentsweep"))
    }

    fn options(&self) -> Options {
        Options {
            data_dir: self.data(),
            socket: self.socket.clone(),
            home: Some(self.root.join("home")),
            key_source: KeySourceChoice::Memory,
            log: LogChoice::Stderr,
        }
    }

    /// A daemon on this sandbox whose Full Scans register the Gate under
    /// `command`, opened and not yet scanning.
    fn open_under(&self, command: GateCommand) -> Arc<Daemon> {
        let daemon = Daemon::open_tuned(
            &self.options(),
            Arc::new(SameUser),
            Box::new(self.keys.clone()),
            Tuning {
                gate: Some(command),
                ..Tuning::default()
            },
        )
        .expect("an open daemon");
        self.started.follow(&daemon);
        daemon
    }

    /// Writes the Gate's entries into both Agents under `command` the way an
    /// earlier version did, without a daemon and without a record.
    fn register_by_hand(&self, command: &GateCommand) {
        let host = Machine::named(self.root.join("home")).host(Vec::new());
        for adapter in both_agents() {
            adapter
                .register_gate(&host, &AtomicWriter, command)
                .expect("the Gate written by hand");
            assert_eq!(adapter.check_gate(&host, command), GateCheck::Registered);
        }
    }

    fn check_under(&self, adapter: &dyn SurfaceAdapter, command: &GateCommand) -> GateCheck {
        let host = Machine::named(self.root.join("home")).host(Vec::new());
        adapter.check_gate(&host, command)
    }

    fn registered_events(&self) -> usize {
        EventLog::in_data_directory(&self.data())
            .read(&self.keys)
            .map(|view| {
                view.events
                    .iter()
                    .filter(|recorded| recorded.event.action == Action::GateRegistered)
                    .count()
            })
            .unwrap_or(0)
    }

    /// True once a Scan has written the state file.
    fn scanned(&self) -> bool {
        self.data().state_file().exists()
    }

    /// A daemon on this sandbox whose Full Scans register the Gate under the
    /// test's command line interface path, with one Full Scan run.
    fn open_with_the_gate(&self) -> Arc<Daemon> {
        let daemon = self.open_under(GateCommand::at(CLI));
        full_scan(&daemon);
        assert_eq!(self.check(&ClaudeCode::new()), GateCheck::Registered);
        assert_eq!(self.check(&Codex::new()), GateCheck::Registered);
        // The socket the daemon would be listening on.
        std::fs::write(&self.socket, b"").expect("a socket stand-in");
        daemon
    }

    fn check(&self, adapter: &dyn SurfaceAdapter) -> GateCheck {
        let host = Machine::named(self.root.join("home")).host(Vec::new());
        adapter.check_gate(&host, &GateCommand::at(CLI))
    }

    fn removed_events(&self) -> usize {
        EventLog::in_data_directory(&self.data())
            .read(&self.keys)
            .expect("a readable Event Log")
            .events
            .iter()
            .filter(|recorded| recorded.event.action == Action::GateRemoved)
            .count()
    }

    /// Puts another tool's handler into the group AgentSweep's
    /// `UserPromptSubmit` handler sits in, so the group is shared.
    fn share_a_group(&self) {
        let mut settings: serde_json::Value =
            serde_json::from_str(&read(&self.claude_settings())).expect("JSON settings");
        let groups = settings["hooks"]["UserPromptSubmit"]
            .as_array_mut()
            .expect("AgentSweep's group");
        groups[0]["hooks"]
            .as_array_mut()
            .expect("its handlers")
            .push(serde_json::json!({"type": "command", "command": FOREIGN}));
        std::fs::write(
            self.claude_settings(),
            serde_json::to_string_pretty(&settings).expect("JSON"),
        )
        .expect("the shared group written");
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        self.started.settle();
        let _ = std::fs::remove_file(&self.socket);
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Claude Code and Codex, the two Agents the Gate registers with.
fn both_agents() -> [Box<dyn SurfaceAdapter>; 2] {
    [Box::new(ClaudeCode::new()), Box::new(Codex::new())]
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("the file")
}

/// Asks for a Full Scan and waits until one has finished.
fn full_scan(daemon: &Arc<Daemon>) {
    daemon.scan(Trigger::ScanNow);
    for _ in 0..12_000 {
        if !daemon.health().scanning && daemon.status().scan.is_some() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    panic!("the Full Scan did not finish");
}

/// Spec section 9.6: a bundle still away at exit takes every AgentSweep
/// handler and pin out of both Agents, one `gate-removed` event each, keeps
/// the user's own hooks and another tool's handler in a shared group, and
/// exits 0.
#[test]
fn a_bundle_gone_for_good_takes_the_gate_out_and_exits_0() {
    let sandbox = Sandbox::new("gone");
    let daemon = sandbox.open_with_the_gate();
    sandbox.share_a_group();
    let before = sandbox.removed_events();

    let code = daemon.leave(Change::Gone, &sandbox.bundle(), &sandbox.socket);

    assert_eq!(code, Some(0));
    assert!(matches!(
        sandbox.check(&ClaudeCode::new()),
        GateCheck::Missing(_)
    ));
    assert!(matches!(
        sandbox.check(&Codex::new()),
        GateCheck::Missing(_)
    ));
    let claude = read(&sandbox.claude_settings());
    assert!(!claude.contains(CLI), "{claude}");
    assert!(claude.contains("bash ~/.claude/lint.sh"), "{claude}");
    assert!(claude.contains(FOREIGN), "{claude}");
    assert_eq!(read(&sandbox.codex_hooks()), USERS_HOOKS);
    assert_eq!(read(&sandbox.codex_config()), USERS_CONFIG, "no pin left");
    assert_eq!(sandbox.removed_events() - before, 5, "one event per entry");
    assert!(!sandbox.socket.exists(), "the socket is unlinked");
}

/// Spec section 9.5: a bundle back at its path by the moment of exit was
/// replaced. The Gate stays, nothing is recorded, and the daemon exits 75 so
/// `KeepAlive` starts the new binary.
#[test]
fn a_bundle_back_by_exit_keeps_the_gate_and_exits_75() {
    let sandbox = Sandbox::new("back");
    let daemon = sandbox.open_with_the_gate();
    std::fs::create_dir_all(sandbox.bundle()).expect("the new bundle in place");
    let claude = read(&sandbox.claude_settings());
    let codex = read(&sandbox.codex_hooks());
    let config = read(&sandbox.codex_config());
    let before = sandbox.removed_events();

    let code = daemon.leave(Change::Gone, &sandbox.bundle(), &sandbox.socket);

    assert_eq!(code, Some(75));
    assert_eq!(read(&sandbox.claude_settings()), claude);
    assert_eq!(read(&sandbox.codex_hooks()), codex);
    assert_eq!(read(&sandbox.codex_config()), config);
    assert_eq!(sandbox.removed_events(), before);

    // A bundle the watch judged replaced keeps the Gate too.
    let code = daemon.leave(Change::Replaced, &sandbox.bundle(), &sandbox.socket);
    assert_eq!(code, Some(75));
    assert_eq!(read(&sandbox.claude_settings()), claude);
    assert_eq!(sandbox.removed_events(), before);
}

/// The hand check of 2026-11-17: a daemon whose command line interface sits
/// in a bundle in the Trash never writes the Gate's entries, however many
/// Full Scans it runs, and records no `gate-registered` event.
#[test]
fn a_daemon_never_registers_the_gate_from_a_trash_path() {
    let sandbox = Sandbox::new("never");
    let command = sandbox.trashed_cli();
    let claude = read(&sandbox.claude_settings());
    let codex = read(&sandbox.codex_hooks());
    let config = read(&sandbox.codex_config());

    let daemon = sandbox.open_under(command.clone());
    full_scan(&daemon);
    full_scan(&daemon);

    for adapter in both_agents() {
        assert!(
            matches!(
                sandbox.check_under(&*adapter, &command),
                GateCheck::Missing(_)
            ),
            "{}",
            adapter.descriptor().agent
        );
    }
    assert_eq!(read(&sandbox.claude_settings()), claude);
    assert_eq!(read(&sandbox.codex_hooks()), codex);
    assert_eq!(read(&sandbox.codex_config()), config);
    assert_eq!(sandbox.registered_events(), 0);
}

/// What a daemon started from the Trash does instead of starting: it takes
/// the Gate's entries under the trashed bundle's command line interface out
/// of both Agents, one `gate-removed` event each, keeps the user's own hooks,
/// and opens no daemon, so it binds no socket and scans nothing.
#[test]
fn leaving_the_trash_takes_the_gate_out_without_starting() {
    let sandbox = Sandbox::new("leave");
    let command = sandbox.trashed_cli();
    sandbox.register_by_hand(&command);

    let removal = agentsweep_daemon::service::leave_the_trash(
        &sandbox.options(),
        &sandbox.trashed_bundle(),
        &sandbox.keys,
        Some(&command),
    );

    assert_eq!(removal.removed, 5, "{removal:?}");
    assert!(removal.failed.is_empty(), "{removal:?}");
    for adapter in both_agents() {
        assert!(matches!(
            sandbox.check_under(&*adapter, &command),
            GateCheck::Missing(_)
        ));
    }
    assert!(read(&sandbox.claude_settings()).contains("bash ~/.claude/lint.sh"));
    assert_eq!(read(&sandbox.codex_hooks()), USERS_HOOKS);
    assert_eq!(read(&sandbox.codex_config()), USERS_CONFIG, "no pin left");
    assert_eq!(sandbox.removed_events(), 5, "one event per entry");
    assert!(!sandbox.socket.exists(), "no socket was bound");
    assert!(!sandbox.scanned(), "no Scan wrote a state file");
}

/// The child process a test started, killed when the test is over whatever
/// happened.
struct Process(std::process::Child);

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The shipped binary, started from a bundle in the Trash the way launchd
/// starts it after a log out and in: it takes the Gate out and exits 0 at
/// once, without listening and without a Full Scan, so `KeepAlive` leaves it
/// down.
#[test]
fn a_daemon_started_from_the_trash_takes_the_gate_out_and_exits_0() {
    use std::io::Read as _;
    use std::process::{Command, Stdio};

    let sandbox = Sandbox::new("started");
    let macos = sandbox.trashed_bundle().join("Contents/MacOS");
    std::fs::create_dir_all(&macos).expect("a trashed bundle");
    let daemon = macos.join("agentsweepd");
    let built = Path::new(env!("CARGO_BIN_EXE_agentsweepd"));
    if std::fs::hard_link(built, &daemon).is_err() {
        std::fs::copy(built, &daemon).expect("the daemon in the trashed bundle");
    }
    let command = sandbox.trashed_cli();
    sandbox.register_by_hand(&command);

    let mut child = Process(
        Command::new(&daemon)
            .arg("--data-dir")
            .arg(sandbox.data().root())
            .arg("--socket")
            .arg(&sandbox.socket)
            .arg("--home")
            .arg(sandbox.root.join("home"))
            .args(["--key-source", "memory", "--log", "stderr"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the daemon starts"),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.0.try_wait().expect("the daemon's status") {
            break status;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the daemon started from the Trash did not exit"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let mut said = String::new();
    if let Some(mut stderr) = child.0.stderr.take() {
        let _ = stderr.read_to_string(&mut said);
    }

    assert_eq!(status.code(), Some(0), "{said}");
    assert!(said.contains("is in the Trash"), "{said}");
    assert!(!said.contains("listening on"), "{said}");
    for adapter in both_agents() {
        assert!(matches!(
            sandbox.check_under(&*adapter, &command),
            GateCheck::Missing(_)
        ));
    }
    assert_eq!(read(&sandbox.codex_config()), USERS_CONFIG, "no pin left");
    assert!(!sandbox.socket.exists(), "no socket was bound");
    assert!(!sandbox.scanned(), "no Scan wrote a state file");
}
