//! Protocol module for PeerUP.
//!
//! This module contains all protocol-related types and implementations.

pub mod codec;
pub mod types;

pub use codec::ProbeCodec;
pub use types::{ProbeRequest, ProbeResponse};

/// Protocol name for probe requests/responses
pub const PROBE_PROTOCOL: &str = "/peerup/probe/1.0";
