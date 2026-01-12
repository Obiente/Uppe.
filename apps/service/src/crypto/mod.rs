pub mod keys;
/// Cryptographic operations for signing and verifying monitoring results
///
/// This module provides:
/// - Ed25519 key generation and management
/// - Signing of monitoring results
/// - Verification of peer results
pub mod signing;
pub mod verification;

pub use keys::{KeyPair, load_or_generate_keypair};
pub use signing::sign_result;
pub use verification::verify_result;
