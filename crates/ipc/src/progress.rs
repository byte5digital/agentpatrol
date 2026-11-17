//! How far a Full Scan has got, as the `scan-progress` state event carries it
//! (spec section 6.3): the phase it is in, and the Agent or the project in
//! hand. The daemon sends it during every Full Scan, a few times a second at
//! most; a Change Scan sends none. Nothing of it is written anywhere.
//!
//! The project is named by its root, which is a path; the app decides how a
//! Posture says it, and the Simple Posture never shows the path.

use serde_json::{Map, Value};

/// Where a Full Scan is.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScanPhase {
    /// It is enumerating: it does not know yet how many Items there are.
    #[default]
    Enumerating,
    /// It knows the total and runs the Tiers over the Items.
    Checking,
}

impl ScanPhase {
    /// The spelling the wire carries.
    pub fn as_str(self) -> &'static str {
        match self {
            ScanPhase::Enumerating => "enumerating",
            ScanPhase::Checking => "checking",
        }
    }

    /// Reads back that spelling.
    pub fn parse(text: &str) -> Option<ScanPhase> {
        match text {
            "enumerating" => Some(ScanPhase::Enumerating),
            "checking" => Some(ScanPhase::Checking),
            _ => None,
        }
    }

    /// The phase an event from before the field existed stands for: no
    /// total yet is still enumerating.
    pub fn of_total(total: usize) -> ScanPhase {
        match total {
            0 => ScanPhase::Enumerating,
            _ => ScanPhase::Checking,
        }
    }
}

/// What a Full Scan has in hand: the Agent whose Surfaces it reads, and the
/// project root when the Items are a project's.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InHand {
    /// The machine-readable Agent id, as the adapter declares it.
    pub agent: String,
    /// The name the adapter gives the Agent for a person.
    pub display_name: String,
    /// The project root, absent for the user's own and the Managed Scope.
    pub project: Option<String>,
}

impl InHand {
    /// The object the wire carries.
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        map.insert("agent".to_string(), Value::from(self.agent.as_str()));
        map.insert(
            "display_name".to_string(),
            Value::from(self.display_name.as_str()),
        );
        if let Some(project) = &self.project {
            map.insert("project".to_string(), Value::from(project.as_str()));
        }
        Value::Object(map)
    }

    /// Reads back that object.
    pub fn from_value(value: &Value) -> Option<InHand> {
        let text = |field: &str| value.get(field).and_then(Value::as_str);
        Some(InHand {
            agent: text("agent")?.to_string(),
            display_name: text("display_name").unwrap_or_default().to_string(),
            project: text("project").map(str::to_string),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phase_reads_back_and_an_old_event_is_read_from_its_total() {
        for phase in [ScanPhase::Enumerating, ScanPhase::Checking] {
            assert_eq!(ScanPhase::parse(phase.as_str()), Some(phase));
        }
        assert_eq!(ScanPhase::parse("nonsense"), None);
        assert_eq!(ScanPhase::of_total(0), ScanPhase::Enumerating);
        assert_eq!(ScanPhase::of_total(3), ScanPhase::Checking);
    }

    #[test]
    fn what_is_in_hand_reads_back_with_and_without_a_project() {
        for project in [None, Some("/tmp/project.invalid/app".to_string())] {
            let in_hand = InHand {
                agent: "claude-code".to_string(),
                display_name: "Claude Code".to_string(),
                project,
            };
            assert_eq!(InHand::from_value(&in_hand.to_value()), Some(in_hand));
        }
    }
}
