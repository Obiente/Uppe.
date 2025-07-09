//! HTTP probe request handling implementation.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tracing::info;

use super::extract::extract_response_headers;
use crate::protocol::{ProbeRequest, ProbeResponse};

/// Handle an HTTP probe request
pub async fn handle_probe_request(request: ProbeRequest) -> ProbeResponse {
    info!("Handling probe request: {} {}", request.method, request.target_url);

    // Record start time
    let start = Instant::now();

    // Get current timestamp
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    // Perform the HTTP request
    let result = perform_http_request(&request).await;

    // Calculate duration
    let duration = start.elapsed().as_millis() as u64;

    // Build response
    match result {
        Ok((status, headers)) => {
            ProbeResponse {
                status: Some(status),
                duration,
                error: None,
                probed_by: "local".to_string(), // This should be the actual peer ID
                timestamp,
                headers: Some(headers),
                body: None, // Added missing field
            }
        }
        Err(error) => {
            ProbeResponse {
                status: None,
                duration,
                error: Some(error.to_string()),
                probed_by: "local".to_string(),
                timestamp,
                headers: None,
                body: None, // Added missing field
            }
        }
    }
}

/// Perform the actual HTTP request
pub async fn perform_http_request(request: &ProbeRequest) -> Result<(u16, Vec<(String, String)>)> {
    // Build HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(request.timeout))
        .user_agent("peerup/1.0")
        .build()?;

    // Prepare the HTTP request
    let mut http_request = match request.method.to_uppercase().as_str() {
        "GET" => client.get(&request.target_url),
        "POST" => client.post(&request.target_url),
        "PUT" => client.put(&request.target_url),
        "DELETE" => client.delete(&request.target_url),
        "HEAD" => client.head(&request.target_url),
        _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", request.method)),
    };

    // Add headers if present
    if let Some(headers) = &request.headers {
        for (key, value) in headers {
            http_request = http_request.header(key, value);
        }
    }

    // Add body if present
    if let Some(body) = &request.body {
        http_request = http_request.body(body.clone());
    }

    // Execute the request
    let response = http_request.send().await?;

    // Extract status code
    let status = response.status().as_u16();

    // Extract response headers (limited set)
    let headers = extract_response_headers(&response);

    Ok((status, headers))
}
