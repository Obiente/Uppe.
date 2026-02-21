use std::time::SystemTime;

use tokio::sync::broadcast;
use tracing::debug;

use crate::database::models::{NetworkStats, Peer};
use crate::p2p::messages::DhtSnapshot;

#[derive(Debug, Clone)]
pub enum TuiEvent {
    Peers(Vec<Peer>),
    NetworkStats(NetworkStats),
    DhtSnapshot(DhtSnapshot),
    DhtQuery(String),
    DhtQueryResult { key: String, ok: bool, #[allow(dead_code)] bytes: Option<Vec<u8>> },
}

static BUS_TX: std::sync::OnceLock<broadcast::Sender<TuiEvent>> = std::sync::OnceLock::new();

fn bus() -> &'static broadcast::Sender<TuiEvent> {
    BUS_TX.get_or_init(|| {
        let (tx, _rx) = broadcast::channel::<TuiEvent>(64);
        tx
    })
}

pub fn subscribe() -> broadcast::Receiver<TuiEvent> { bus().subscribe() }

pub fn publish_peers(peers: Vec<String>, now: SystemTime) {
    let peer_models: Vec<Peer> = peers
        .into_iter()
        .map(|id| Peer::new_online(id, now))
        .collect();
    debug!(count = peer_models.len(), "TUI bus: publishing peers update");
    publish(TuiEvent::Peers(peer_models));
}

pub fn publish_network_stats(stats: NetworkStats) { publish(TuiEvent::NetworkStats(stats)); }

pub fn publish_dht_snapshot(snapshot: DhtSnapshot) {
    debug!(buckets = snapshot.buckets.len(), "TUI bus: publishing DHT snapshot");
    publish(TuiEvent::DhtSnapshot(snapshot));
}

pub fn publish_dht_query(key: String) {
    debug!(key = %key, "TUI bus: publishing DHT GET request");
    publish(TuiEvent::DhtQuery(key));
}

pub fn publish_dht_query_result(key: String, ok: bool, bytes: Option<Vec<u8>>) {
    debug!(key = %key, ok, size = bytes.as_ref().map(|b| b.len()), "TUI bus: publishing DHT GET result");
    publish(TuiEvent::DhtQueryResult { key, ok, bytes });
}

fn publish(ev: TuiEvent) {
    // Ignore errors if there are no receivers
    let _ = bus().send(ev);
}
