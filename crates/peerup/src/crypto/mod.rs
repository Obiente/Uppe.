//! Cryptographic operations for PeerUP.
//!
//! This module is the single source of truth for all crypto in the Uppe ecosystem:
//! - Ed25519 key generation, signing, and verification
//! - X25519 + XChaCha20-Poly1305 encryption for private data
//! - Signed result verification for P2P distribution

pub mod encryption;
pub mod keys;
pub mod signatures;
pub mod signing;
pub mod verification;

pub use encryption::{
    decrypt_result_for_owner, encrypt_result_for_owner, EncryptedResult, EncryptedResultBatch,
};
pub use keys::{generate_keypair, load_or_generate_keypair, KeyPair};
pub use signatures::{verify_received_result, SignedResult};
pub use signing::{sign_bytes, sign_json};
pub use verification::verify_signature;
