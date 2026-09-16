//! Uppe-specific public monitor orchestration.
//!
//! `peerup` provides generic peer transport, discovery, storage and crypto.
//! Public monitor identity, schedules, votes and wire messages are application
//! policy, so they live with the service orchestrator instead of the peer crate.

mod consensus;
mod messages;
mod visibility;

pub use consensus::{ConsensusManager, OrchestrationVote};
pub use messages::PublicMonitorMessage;
pub use visibility::{OrchestrationSchedule, PublicMonitorGroup};
