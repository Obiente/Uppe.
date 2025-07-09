//! Protocol module for PeerUP.
//!
//! This module contains all protocol-related types and implementations.

pub mod types;
pub mod codec;

pub use types::{ProbeRequest, ProbeResponse};
pub use codec::ProbeCodec;

/// Protocol name for probe requests/responses
pub const PROBE_PROTOCOL: &str = "/peerup/probe/1.0";
