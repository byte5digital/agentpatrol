//! The eval bar of spec section 4.4, in one place.
//!
//! The eval measures each Judgement Engine and model against the benign
//! corpus under one `prompt_version`. [`MEASURED`] holds a row per measured
//! engine, model and `prompt_version` with its standing: within the bar or
//! outside it. An engine and model with no row under the `prompt_version`
//! in force is not measured, so a prompt change, which raises
//! `prompt_version`, leaves every engine not measured until the eval runs
//! again. Only an engine within the bar may be on in the Simple Posture: the
//! daemon, the only writer of the settings, refuses to switch any other one
//! on there, and switches one off when the Posture moves to Simple. The
//! Developer Posture may switch any engine on, with a warning.
//!
//! A row names the model the way the eval measured it: the Ollama tag (any
//! digest after `@` is not compared), the Claude model id, and for Apple
//! on-device the major macOS version, since an Apple row holds for every
//! release of the major version it was measured on and a new major version
//! is not measured until a release measures it.

use crate::judgement::PROMPT_VERSION;
use crate::settings::{JudgementEngine, JudgementSettings, Posture};

/// Where an engine and model stand against the bar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Standing {
    /// Measured, and within the bar.
    Within,
    /// Measured, and outside the bar.
    Outside,
    /// No row under the `prompt_version` in force.
    NotMeasured,
}

impl Standing {
    /// The standing as the wire spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Standing::Within => "within",
            Standing::Outside => "outside",
            Standing::NotMeasured => "not-measured",
        }
    }

    /// Reads back what [`Standing::as_str`] wrote.
    pub fn parse(text: &str) -> Option<Standing> {
        [Standing::Within, Standing::Outside, Standing::NotMeasured]
            .into_iter()
            .find(|standing| standing.as_str() == text)
    }
}

/// One measured engine, model and `prompt_version`, and its standing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Row {
    /// The engine.
    pub engine: JudgementEngine,
    /// The model: the Ollama tag, the Claude model id, or the major macOS
    /// version for Apple on-device.
    pub model: &'static str,
    /// The `prompt_version` the eval ran under.
    pub prompt_version: &'static str,
    /// Within the bar or outside it.
    pub standing: Standing,
}

/// Every measured engine and model, with its standing, from the eval runs of
/// 2026-11-24 (docs/evaluations.md). Apple and Ollama are outside the bar.
/// Claude called no benign Item malicious; the benign corpus holds 29 Items
/// in the Ambiguous Band, one short of the floor of thirty, and the owner
/// accepted it as within the bar all the same.
pub const MEASURED: &[Row] = &[
    Row {
        engine: JudgementEngine::Apple,
        model: "26",
        prompt_version: "2",
        standing: Standing::Outside,
    },
    Row {
        engine: JudgementEngine::Ollama,
        model: "gemma3:27b",
        prompt_version: "2",
        standing: Standing::Outside,
    },
    Row {
        engine: JudgementEngine::Claude,
        model: "claude-sonnet-5",
        prompt_version: "2",
        standing: Standing::Within,
    },
];

/// The major version of a macOS version string such as `26.6.2 (25G83)`.
pub fn macos_major(version: &str) -> Option<&str> {
    let major = version.trim().split(['.', ' ']).next()?;
    (!major.is_empty() && major.bytes().all(|byte| byte.is_ascii_digit())).then_some(major)
}

/// The model as a row names it: the major macOS version for Apple, the tag
/// without any `@digest` for Ollama, the model id for Claude. None for off,
/// and for an Apple version that names no major version.
fn row_model(engine: JudgementEngine, model: &str) -> Option<&str> {
    match engine {
        JudgementEngine::Off => None,
        JudgementEngine::Apple => macos_major(model),
        JudgementEngine::Ollama => model.split('@').next(),
        JudgementEngine::Claude => Some(model),
    }
}

/// Where `engine` running `model` stands in `table` under `prompt_version`.
/// For Apple, `model` is the macOS version the engine reports; for Ollama
/// any `@digest` is ignored. Off is never measured.
pub fn standing_in(
    table: &[Row],
    prompt_version: &str,
    engine: JudgementEngine,
    model: &str,
) -> Standing {
    let Some(model) = row_model(engine, model) else {
        return Standing::NotMeasured;
    };
    table
        .iter()
        .find(|row| {
            row.engine == engine && row.model == model && row.prompt_version == prompt_version
        })
        .map_or(Standing::NotMeasured, |row| row.standing)
}

/// Where `engine` running `model` stands under the `prompt_version` in
/// force.
pub fn standing(engine: JudgementEngine, model: &str) -> Standing {
    standing_in(MEASURED, PROMPT_VERSION, engine, model)
}

/// Whether `engine` running `model` is within the bar in `table` under the
/// `prompt_version` in force.
pub fn passes_in(table: &[Row], engine: JudgementEngine, model: &str) -> bool {
    standing_in(table, PROMPT_VERSION, engine, model) == Standing::Within
}

/// Whether `engine` running `model` cleared the bar.
pub fn passes(engine: JudgementEngine, model: &str) -> bool {
    passes_in(MEASURED, engine, model)
}

/// True when `table` holds a row within the bar for `engine` under the
/// `prompt_version` in force, so its model is worth finding out.
pub fn has_passing_rows_in(table: &[Row], engine: JudgementEngine) -> bool {
    table.iter().any(|row| {
        row.engine == engine
            && row.prompt_version == PROMPT_VERSION
            && row.standing == Standing::Within
    })
}

/// True when any engine in `table` is within the bar under the
/// `prompt_version` in force.
pub fn any_passes_in(table: &[Row]) -> bool {
    JudgementEngine::ALL
        .into_iter()
        .any(|engine| has_passing_rows_in(table, engine))
}

/// True when any engine cleared the bar at all.
pub fn any_passes() -> bool {
    any_passes_in(MEASURED)
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
    admits_in(MEASURED, posture, judgement, apple_version)
}

/// [`admits`] against `table`.
pub fn admits_in(
    table: &[Row],
    posture: Posture,
    judgement: &JudgementSettings,
    apple_version: impl FnOnce() -> Option<String>,
) -> bool {
    if posture == Posture::Developer || !judgement.engine.is_on() {
        return true;
    }
    let model = match judgement.engine {
        JudgementEngine::Apple if has_passing_rows_in(table, JudgementEngine::Apple) => {
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

    const fn within(engine: JudgementEngine, model: &'static str) -> Row {
        Row {
            engine,
            model,
            prompt_version: PROMPT_VERSION,
            standing: Standing::Within,
        }
    }

    const TABLE: &[Row] = &[
        within(JudgementEngine::Apple, "26"),
        within(JudgementEngine::Ollama, "qwen3:8b"),
        within(JudgementEngine::Claude, "claude-sonnet-5"),
        Row {
            engine: JudgementEngine::Ollama,
            model: "gemma3:27b",
            prompt_version: PROMPT_VERSION,
            standing: Standing::Outside,
        },
    ];

    /// The eval runs of 2026-11-24 measured one row per engine under
    /// prompt version 2: Apple and Ollama are outside the bar, Claude within
    /// it by the owner's decision.
    #[test]
    fn the_measured_rows_stand_as_the_eval_runs_left_them() {
        assert_eq!(
            PROMPT_VERSION, "2",
            "a new prompt version needs new eval runs"
        );
        assert!(any_passes());
        for engine in JudgementEngine::ALL {
            assert_eq!(
                has_passing_rows_in(MEASURED, engine),
                engine == JudgementEngine::Claude
            );
        }
        assert_eq!(
            standing(JudgementEngine::Apple, "26.6.2 (25G83)"),
            Standing::Outside
        );
        assert_eq!(
            standing(JudgementEngine::Apple, "26.0 (25A1)"),
            Standing::Outside
        );
        assert_eq!(
            standing(JudgementEngine::Apple, "27.0 (26A1)"),
            Standing::NotMeasured
        );
        assert_eq!(
            standing(
                JudgementEngine::Ollama,
                "gemma3:27b@a418f5838eaf7fe2cfe0a3046c8384b68ba43a4435542c942f9db00a5f342203"
            ),
            Standing::Outside
        );
        assert_eq!(
            standing(JudgementEngine::Ollama, "qwen3:8b"),
            Standing::NotMeasured
        );
        assert_eq!(
            standing(JudgementEngine::Claude, "claude-sonnet-5"),
            Standing::Within
        );
        assert_eq!(
            standing(JudgementEngine::Claude, "claude-custom"),
            Standing::NotMeasured
        );
        for engine in JudgementEngine::ALL {
            assert!(!passes(engine, "26.6.2 (25G83)"));
            assert!(!passes(engine, "gemma3:27b"));
            assert_eq!(
                passes(engine, "claude-sonnet-5"),
                engine == JudgementEngine::Claude
            );
        }
    }

    /// A prompt change raises `prompt_version`, and the table has no row for
    /// the new version until the eval runs again: every engine and model is
    /// not measured under it, whatever it stood at before.
    #[test]
    fn a_prompt_version_with_no_row_is_not_measured_for_every_engine() {
        for table in [TABLE, MEASURED] {
            for engine in JudgementEngine::ALL {
                for model in [
                    "26.6.2 (25G83)",
                    "qwen3:8b",
                    "gemma3:27b",
                    "claude-sonnet-5",
                    "",
                ] {
                    assert_eq!(
                        standing_in(table, "next", engine, model),
                        Standing::NotMeasured,
                        "{engine} {model}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_standing_round_trips_through_its_name() {
        for standing in [Standing::Within, Standing::Outside, Standing::NotMeasured] {
            assert_eq!(Standing::parse(standing.as_str()), Some(standing));
        }
        assert_eq!(Standing::parse("passing"), None);
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
        assert_eq!(
            standing_in(
                TABLE,
                PROMPT_VERSION,
                JudgementEngine::Ollama,
                "gemma3:27b@1111"
            ),
            Standing::Outside
        );
        assert!(!passes_in(TABLE, JudgementEngine::Ollama, "gemma3:27b"));
        assert!(passes_in(TABLE, JudgementEngine::Claude, "claude-sonnet-5"));
        assert!(!passes_in(TABLE, JudgementEngine::Claude, "qwen3:8b"));
        assert!(!passes_in(TABLE, JudgementEngine::Off, ""));
        assert_eq!(
            standing_in(TABLE, PROMPT_VERSION, JudgementEngine::Off, ""),
            Standing::NotMeasured
        );
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
        let outside = with(JudgementEngine::Ollama, "gemma3:27b");
        let apple = with(JudgementEngine::Apple, "");

        assert!(admits_in(TABLE, Posture::Simple, &off, never));
        assert!(admits_in(TABLE, Posture::Simple, &tested, never));
        assert!(!admits_in(TABLE, Posture::Simple, &untested, never));
        assert!(!admits_in(TABLE, Posture::Simple, &outside, never));
        assert!(admits_in(TABLE, Posture::Developer, &untested, never));
        assert!(admits_in(TABLE, Posture::Developer, &outside, never));
        assert!(admits_in(TABLE, Posture::Simple, &apple, || Some(
            "26.1 (25B1)".to_string()
        )));
        assert!(!admits_in(TABLE, Posture::Simple, &apple, || None));
        // With no Apple row within the bar, the version is never asked.
        assert!(!admits_in(&[], Posture::Simple, &apple, never));
        assert!(!admits(Posture::Simple, &apple, never));
        assert!(admits(Posture::Developer, &apple, never));
        let claude = with(JudgementEngine::Claude, "");
        assert!(admits(Posture::Simple, &claude, never));
    }
}
