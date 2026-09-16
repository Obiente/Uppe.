use serde::{Deserialize, Serialize};

use super::{OrchestrationSchedule, OrchestrationVote, PublicMonitorGroup};

/// Wire messages for Uppe's community-owned public monitor registry.
/// These belong to the application orchestrator, while `peerup` only carries
/// generic peer data and transport messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublicMonitorMessage {
    Vote { vote: Box<OrchestrationVote> },
    Announce { domain: String, display_name: String, creator_peer_id: String },
    Join { domain: String, peer_id: String },
    Leave { domain: String, peer_id: String },
    ScheduleUpdate { domain: String, schedule: OrchestrationSchedule },
    GroupQuery { domain: String },
    GroupResponse { domain: String, group: Option<PublicMonitorGroup> },
}

impl PublicMonitorMessage {
    pub fn domain(&self) -> &str {
        match self {
            Self::Vote { vote } => &vote.domain,
            Self::Announce { domain, .. }
            | Self::Join { domain, .. }
            | Self::Leave { domain, .. }
            | Self::ScheduleUpdate { domain, .. }
            | Self::GroupQuery { domain }
            | Self::GroupResponse { domain, .. } => domain,
        }
    }

    pub fn peer_id(&self) -> Option<&str> {
        match self {
            Self::Vote { vote } => Some(&vote.voter_peer_id),
            Self::Announce { creator_peer_id, .. } => Some(creator_peer_id),
            Self::Join { peer_id, .. } | Self::Leave { peer_id, .. } => Some(peer_id),
            _ => None,
        }
    }
}
