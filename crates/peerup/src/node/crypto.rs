//! Cryptographic utilities for PeerUP.
//!
//! This module handles keypair generation and management.

use std::{fs, path::Path};

use anyhow::Result;
use libp2p::identity::Keypair;

/// Load or generate a keypair from the specified path
pub fn load_or_generate_keypair<P: AsRef<Path,>,>(path: P,) -> Result<Keypair,> {
    let path = path.as_ref();

    if path.exists() {
        // Load existing keypair
        let bytes = fs::read(path,)?;
        let keypair = Keypair::from_protobuf_encoding(&bytes,)?;
        Ok(keypair,)
    } else {
        // Generate new keypair
        let keypair = Keypair::generate_ed25519();

        // Save to file
        if let Some(parent,) = path.parent() {
            fs::create_dir_all(parent,)?;
        }
        let bytes = keypair.to_protobuf_encoding()?;
        fs::write(path, &bytes,)?;

        Ok(keypair,)
    }
}

/// Generate a new Ed25519 keypair
pub fn generate_keypair() -> Keypair {
    Keypair::generate_ed25519()
}

/// Save a keypair to a file
pub fn save_keypair<P: AsRef<Path,>,>(keypair: &Keypair, path: P,) -> Result<(),> {
    let path = path.as_ref();

    if let Some(parent,) = path.parent() {
        fs::create_dir_all(parent,)?;
    }

    let bytes = keypair.to_protobuf_encoding()?;
    fs::write(path, &bytes,)?;

    Ok((),)
}

/// Load a keypair from a file
pub fn load_keypair<P: AsRef<Path,>,>(path: P,) -> Result<Keypair,> {
    let bytes = fs::read(path,)?;
    let keypair = Keypair::from_protobuf_encoding(&bytes,)?;
    Ok(keypair,)
}
