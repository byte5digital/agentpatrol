//! A Full Scan's progress as its subscribers see it (spec section 6.3): the
//! phase, the Items done of the total once the enumeration knows it, and the
//! Agent or the project in hand.
//!
//! A Scan reports far more often than anyone can read, from several worker
//! threads at once, so [`Throttle`] lets a few events a second through. What
//! always goes out: the first event, a new phase, a new Agent while it
//! enumerates (there are only a few), and the last Item. Nothing of it is
//! written anywhere: not to the Event Log, not to the state file, not to the
//! log.

use std::time::{Duration, Instant};

use agentsweep_core::adapter::Descriptor;
use agentsweep_core::item::{Item, Scope};
use agentsweep_ipc::protocol::{InHand, ScanPhase, StateEvent};

/// The least time between two progress events that carry nothing new but
/// the count: four a second at most.
pub const EVERY: Duration = Duration::from_millis(250);

/// The Agent whose Surfaces a Scan enumerates.
pub fn of_agent(descriptor: &Descriptor) -> InHand {
    InHand {
        agent: descriptor.agent.to_string(),
        display_name: descriptor.display_name.to_string(),
        project: None,
    }
}

/// The Item the Tiers were last done with: its Agent, named as `names` says,
/// and its project root when it is a project's.
pub fn of_item(item: &Item, names: &[(&str, &str)]) -> InHand {
    let display_name = names
        .iter()
        .find(|(agent, _)| *agent == item.agent)
        .map_or(item.agent.as_str(), |(_, name)| name);
    InHand {
        agent: item.agent.clone(),
        display_name: display_name.to_string(),
        project: match &item.scope {
            Scope::Project(root) => Some(root.display().to_string()),
            Scope::User | Scope::Managed => None,
        },
    }
}

/// Which progress events go out.
#[derive(Clone, Debug, Default)]
pub struct Throttle {
    sent: Option<(Instant, StateEvent)>,
}

impl Throttle {
    /// True when `event` goes out at `now`; it is then the last one sent.
    pub fn lets_through(&mut self, now: Instant, event: &StateEvent) -> bool {
        let StateEvent::ScanProgress {
            done,
            total,
            phase,
            in_hand,
        } = event
        else {
            return false;
        };
        let through = match &self.sent {
            None => true,
            Some((_, last)) if last == event => false,
            Some((
                at,
                StateEvent::ScanProgress {
                    phase: last_phase,
                    in_hand: last_in_hand,
                    ..
                },
            )) => {
                last_phase != phase
                    || (*phase == ScanPhase::Enumerating && last_in_hand != in_hand)
                    || (*phase == ScanPhase::Checking && done == total)
                    || now.duration_since(*at) >= EVERY
            }
            Some(_) => true,
        };
        if through {
            self.sent = Some((now, event.clone()));
        }
        through
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use agentsweep_core::item::{Activation, Kind, Provenance};

    fn checking(done: usize, total: usize, project: Option<&str>) -> StateEvent {
        StateEvent::ScanProgress {
            done,
            total,
            phase: ScanPhase::Checking,
            in_hand: Some(InHand {
                agent: "claude-code".to_string(),
                display_name: "Claude Code".to_string(),
                project: project.map(str::to_string),
            }),
        }
    }

    fn enumerating(agent: Option<&str>) -> StateEvent {
        StateEvent::ScanProgress {
            done: 0,
            total: 0,
            phase: ScanPhase::Enumerating,
            in_hand: agent.map(|agent| InHand {
                agent: agent.to_string(),
                display_name: agent.to_string(),
                project: None,
            }),
        }
    }

    /// A few events a second: the count alone goes out once a quarter
    /// second at most, every new phase and Agent at once, the last Item
    /// always, and the same event never twice.
    #[test]
    fn a_few_events_a_second_and_never_the_last_item_lost() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut throttle = Throttle::default();

        assert!(throttle.lets_through(at(0), &enumerating(None)));
        assert!(
            !throttle.lets_through(at(1), &enumerating(None)),
            "the same"
        );
        assert!(throttle.lets_through(at(2), &enumerating(Some("workspace"))));
        assert!(throttle.lets_through(at(3), &enumerating(Some("claude-code"))));
        assert!(throttle.lets_through(at(4), &enumerating(Some("codex"))));

        // The total is known: a new phase.
        assert!(throttle.lets_through(at(5), &checking(0, 600, None)));
        let mut through = 0;
        for done in 1..600 {
            let project = format!("/tmp/project.invalid/{}", done % 7);
            if throttle.lets_through(
                at(5 + done as u64 * 2),
                &checking(done, 600, Some(&project)),
            ) {
                through += 1;
            }
        }
        // 1198 ms of Items: one event every 250 ms at most.
        assert!((4..=5).contains(&through), "{through}");
        assert!(
            throttle.lets_through(
                at(1_204),
                &checking(600, 600, Some("/tmp/project.invalid/1"))
            ),
            "the last Item goes out"
        );
        assert!(
            throttle.lets_through(at(1_205), &checking(600, 600, None)),
            "the end without an Item in hand"
        );
        assert!(!throttle.lets_through(at(1_206), &checking(600, 600, None)));

        // Anything else is not progress.
        assert!(!throttle.lets_through(
            at(2_000),
            &StateEvent::JudgementProgress { done: 1, total: 2 }
        ));
    }

    /// The Item in hand names its Agent as the adapter does and its project
    /// root only for a project's Item.
    #[test]
    fn the_item_in_hand_names_its_agent_and_its_project() {
        let names = [
            ("claude-code", "Claude Code"),
            ("workspace", "Project files"),
        ];
        let mut item = Item::file(
            "claude-code",
            Scope::User,
            Kind::InstructionFile,
            "CLAUDE.md",
            PathBuf::from("/tmp/home.invalid/.claude/CLAUDE.md"),
            Provenance::Manual,
            Activation::Active,
            b"# notes\n".to_vec(),
        );
        assert_eq!(
            of_item(&item, &names),
            InHand {
                agent: "claude-code".to_string(),
                display_name: "Claude Code".to_string(),
                project: None,
            }
        );
        item.scope = Scope::Project(PathBuf::from("/tmp/project.invalid/app"));
        item.agent = "workspace".to_string();
        let in_hand = of_item(&item, &names);
        assert_eq!(in_hand.display_name, "Project files");
        assert_eq!(in_hand.project.as_deref(), Some("/tmp/project.invalid/app"));
        item.agent = "unknown".to_string();
        assert_eq!(of_item(&item, &names).display_name, "unknown");
    }
}
