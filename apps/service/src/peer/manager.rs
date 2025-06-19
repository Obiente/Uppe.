


fn connect_to_peer(peer_address: &str) -> Result<(), String> {
    // Simulate a connection to a peer
    if peer_address.is_empty() {
        return Err("Peer address cannot be empty".to_string());
    }
    println!("Connected to peer at {}", peer_address);
    Ok(())
}