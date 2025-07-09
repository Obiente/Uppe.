//! Protocol type definitions for PeerUP.
//!
//! This module defines the data structures used in the PeerUP protocol.

use serde::{Serialize, Deserialize};

/// A probe request to be sent to a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeRequest {
    /// The URL to probe
    pub target_url: String,
    
    /// The HTTP method to use (GET, POST, etc.)
    pub method: String,
    
    /// Timeout in milliseconds
    pub timeout: u64,
    
    /// Optional request body
    pub body: Option<String>,
    
    /// Optional request headers
    pub headers: Option<Vec<(String, String)>>,
    
    /// Requested by (peer ID)
    pub requested_by: String,
}

/// A probe response from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResponse {
    /// HTTP status code
    pub status: Option<u16>,
    
    /// Duration of the request in milliseconds
    pub duration: u64,
    
    /// Error message if the probe failed
    pub error: Option<String>,
    
    /// The peer that performed the probe
    pub probed_by: String,
    
    /// Timestamp when the probe was performed
    pub timestamp: u64,
    
    /// Response headers (limited set)
    pub headers: Option<Vec<(String, String)>>,

    pub body: Option<String>,
}
