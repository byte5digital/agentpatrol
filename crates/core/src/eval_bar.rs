//! The eval bar of spec section 4.4, in one place.
//!
//! The eval measures each Judgement Engine and model against the benign
//! corpus. An engine that clears the bar is listed in [`PASSING`]; every
//! other engine and model is untested here, whatever the reason. An untested
//! engine stays off in the Simple Posture: the daemon, the only writer of the
//! settings, refuses to switch one on there, and switches one off when the
//! Posture moves to Simple. The Developer Posture may switch any engine on,
//! with a warning.
//!
//! A row names the model the way the eval measured it: the Ollama tag (any
//! digest after `@` is not compared), the Claude model id, and for Apple
//! on-device the major macOS version, since an Apple row holds for every
//! release of the major version it was measured on and a new major version
//! is unmeasured until a release measures it.

use crate::settings::{JudgementEngine, JudgementSettings, Posture};

/// One engine and model that cleared the bar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Passing {
    /// The engine.
    pub engine: JudgementEngine,
    /// The model: the Ollama tag, the Claude model id, or the major macOS
    /// version for Apple on-device.
    pub model: &'static str,
}

/// Every engine and model that cleared the bar. Empty until the eval runs
/// record a passing row; until then no engine passes.
pub const PASSING: &[Passing] = &[];

/// The major version of a macOS version string such as `26.6.2 (25G83)`.
pub fn macos_major(version: &str) -> Option<&str> {
    let major = version.trim().split(['.', ' ']).next()?;
    (!major.is_empty() && major.bytes().all(|byte| byte.is_ascii_digit())).then_some(major)
}

/// Whether `engine` running `model` is in `table`. For Apple, `model` is the
/// macOS version the engine reports; for Ollama any `@digest` is ignored.
pub fn passes_in(table: &[Passing], engine: JudgementEngine, model: &str) -> bool {
    let model = match engine {
        JudgementEngine::Off => return false,
        JudgementEngine::Apple => match macos_major(model) {
            Some(major) => major,
            None => return false,
        },
        JudgementEngine::Ollama => model.split('@').next().unwrap_or(model),
        JudgementEngine::Claude => model,
    };
    table
        .iter()
        .any(|row| row.engine == engine && row.model == model)
}

/// Whether `engine` running `model` cleared the bar.
pub fn passes(engine: JudgementEngine, model: &str) -> bool {
    passes_in(PASSING, engine, model)
}

/// True when the table holds a row for `engine`, so its model is worth
/// finding out.
pub fn has_rows_for(engine: JudgementEngine) -> bool {
    PASSING.iter().any(|row| row.engine == engine)
}

/// True when any engine cleared the bar at all.
pub fn any_passes() -> bool {
    !PASSING.is_empty()
}

/// Whether the AI Judgement choices may stand under `posture`: always in the
/// Developer Posture and with AI Judgement off, else only within the bar.
/// `apple_version` names the macOS version Apple on-device would run under;
/// it is asked only when that is needed.
pub fn admits(
    posture: Posture,
    judgement: &JudgementSettings,
    apple_version: impl FnOnce() -> Option<String>,
) -> bool {
    admits_in(PASSING, posture, judgement, apple_version)
}

/// [`admits`] against `table`.
pub fn admits_in(
    table: &[Passing],
    posture: Posture,
    judgement: &JudgementSettings,
    apple_version: impl FnOnce() -> Option<String>,
) -> bool {
    if posture == Posture::Developer || !judgement.engine.is_on() {
        return true;
    }
    let model = match judgement.engine {
        JudgementEngine::Apple if table.iter().any(|row| row.engine == JudgementEngine::Apple) => {
            apple_version().unwrap_or_default()
        }
        JudgementEngine::Apple => return false,
        engine => judgement.model_of(engine).to_string(),
    };
    passes_in(table, judgement.engine, &model)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &[Passing] = &[
        Passing {
            engine: JudgementEngine::Apple,
            model: "26",
        },
        Passing {
            engine: JudgementEngine::Ollama,
            model: "qwen3:8b",
        },
        Passing {
            engine: JudgementEngine::Claude,
            model: "claude-sonnet-5",
        },
    ];

    #[test]
    fn no_engine_passes_until_the_eval_records_one() {
        assert!(!any_passes());
        for engine in JudgementEngine::ALL {
            assert!(!has_rows_for(engine));
            assert!(!passes(engine, "26.6.2 (25G83)"));
            assert!(!passes(engine, "qwen3:8b"));
            assert!(!passes(engine, "claude-sonnet-5"));
        }
    }

    #[test]
    fn an_apple_row_holds_for_its_major_macos_version_only() {
        assert_eq!(macos_major("26.6.2 (25G83)"), Some("26"));
        assert_eq!(macos_major("27 (26A1)"), Some("27"));
        assert_eq!(macos_major(""), None);
        assert_eq!(macos_major("beta"), None);
        assert!(passes_in(TABLE, JudgementEngine::Apple, "26.0 (25A1)"));
        assert!(passes_in(TABLE, JudgementEngine::Apple, "26.6.2 (25G83)"));
        assert!(!passes_in(TABLE, JudgementEngine::Apple, "27.0 (26A1)"));
        assert!(!passes_in(TABLE, JudgementEngine::Apple, ""));
    }

    #[test]
    fn an_ollama_row_names_the_tag_and_a_claude_row_the_model_id() {
        assert!(passes_in(TABLE, JudgementEngine::Ollama, "qwen3:8b"));
        assert!(passes_in(
            TABLE,
            JudgementEngine::Ollama,
            "qwen3:8b@sha256:0000"
        ));
        assert!(!passes_in(TABLE, JudgementEngine::Ollama, "qwen3:4b"));
        assert!(passes_in(TABLE, JudgementEngine::Claude, "claude-sonnet-5"));
        assert!(!passes_in(TABLE, JudgementEngine::Claude, "qwen3:8b"));
        assert!(!passes_in(TABLE, JudgementEngine::Off, ""));
    }

    #[test]
    fn simple_admits_only_what_passes_and_developer_everything() {
        let with = |engine, model: &str| JudgementSettings {
            engine,
            ollama_model: model.to_string(),
            ..JudgementSettings::default()
        };
        let never = || -> Option<String> { panic!("the Apple version is not needed here") };
        let off = with(JudgementEngine::Off, "");
        let tested = with(JudgementEngine::Ollama, "qwen3:8b");
        let untested = with(JudgementEngine::Ollama, "qwen3:4b");
        let apple = with(JudgementEngine::Apple, "");

        assert!(admits_in(TABLE, Posture::Simple, &off, never));
        assert!(admits_in(TABLE, Posture::Simple, &tested, never));
        assert!(!admits_in(TABLE, Posture::Simple, &untested, never));
        assert!(admits_in(TABLE, Posture::Developer, &untested, never));
        assert!(admits_in(TABLE, Posture::Simple, &apple, || Some(
            "26.1 (25B1)".to_string()
        )));
        assert!(!admits_in(TABLE, Posture::Simple, &apple, || None));
        // With no Apple row, the version is never asked.
        assert!(!admits_in(&[], Posture::Simple, &apple, never));
        assert!(!admits(Posture::Simple, &apple, never));
        assert!(admits(Posture::Developer, &apple, never));
    }
}
