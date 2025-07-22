//! Tests for node crypto operations

use peerup::node::{generate_keypair, peer_id_from_keypair};
use libp2p::identity::Keypair;

#[test]
fn test_generate_keypair() {
    let keypair = generate_keypair();
    // Accept any keypair type, as Ed25519 variant may not be public in new libp2p
    let keypair_type = format!("{:?}", keypair);
    assert!(keypair_type.contains("Ed25519") || keypair_type.contains("ed25519"), "Expected Ed25519 keypair");
}

#[test]
fn test_peer_id_from_keypair() {
    let keypair = generate_keypair();
    let peer_id = peer_id_from_keypair(&keypair);
    
    // PeerID should be consistent
    let peer_id2 = peer_id_from_keypair(&keypair);
    assert_eq!(peer_id, peer_id2);
}

#[test]
fn test_different_keypairs_different_peer_ids() {
    let keypair1 = generate_keypair();
    let keypair2 = generate_keypair();
    
    let peer_id1 = peer_id_from_keypair(&keypair1);
    let peer_id2 = peer_id_from_keypair(&keypair2);
    
    assert_ne!(peer_id1, peer_id2);
}
