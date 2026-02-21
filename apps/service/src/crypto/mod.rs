/// Cryptographic operations for the Uppe service.
///
/// Core crypto (keys, encryption, signing, verification) lives in the
/// `peerup` crate. This module re-exports those and provides
/// service-specific wrappers that depend on Uppe types like `CheckResult`.

// Service-specific wrappers (depend on CheckResult / PeerResult)
pub mod signing;
pub mod verification;

// Re-export from peerup's crypto module
pub use peerup::crypto::{
    decrypt_result_for_owner, encrypt_result_for_owner, load_or_generate_keypair, EncryptedResult,
    KeyPair,
};

// Re-export service-specific functions
pub use signing::sign_result;
pub use verification::verify_result;
