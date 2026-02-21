//! Conversions from Kademlia events to PeerUPEvent.

use libp2p::kad;

use crate::network::events::PeerUPEvent;

impl From<kad::Event> for PeerUPEvent {
    fn from(event: kad::Event) -> Self {
        use kad::{Event::*, QueryResult::*};
        match event {
            OutboundQueryProgressed { result, .. } => match result {
                GetClosestPeers(Ok(peers)) => {
                    if let Some(peer_info) = peers.peers.into_iter().next() {
                        PeerUPEvent::PeerDiscovered(peer_info.peer_id)
                    } else {
                        PeerUPEvent::Noop
                    }
                }
                GetRecord(Ok(ok)) => match ok {
                    kad::GetRecordOk::FoundRecord(peer_record) => {
                        let key = peer_record.record.key.as_ref().to_vec();
                        let value = peer_record.record.value.clone();
                        PeerUPEvent::DhtGetRecordOk { key, record: value }
                    }
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. } => {
                        PeerUPEvent::DhtGetRecordErr { key: Vec::new() }
                    }
                },
                GetRecord(Err(_e)) => PeerUPEvent::DhtGetRecordErr { key: Vec::new() },
                PutRecord(Ok(ok)) => {
                    let key = ok.key.as_ref().to_vec();
                    PeerUPEvent::DhtPutRecordOk { key }
                }
                PutRecord(Err(e)) => {
                    PeerUPEvent::DhtPutRecordErr {
                        key: Vec::new(),
                        error: format!("{}", e),
                    }
                }
                _ => PeerUPEvent::Noop,
            },
            RoutingUpdated { peer, .. } | PendingRoutablePeer { peer, .. } => {
                PeerUPEvent::PeerDiscovered(peer)
            }
            _ => PeerUPEvent::Noop,
        }
    }
}
