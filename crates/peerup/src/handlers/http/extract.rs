//! HTTP response extraction and handling.

/// Extract important response headers
pub fn extract_response_headers(response: &reqwest::Response,) -> Vec<(String, String,),> {
    let mut headers = Vec::new();

    // Extract commonly useful headers
    let header_names = [
        "content-type",
        "content-length",
        "server",
        "cache-control",
        "date",
        "last-modified",
        "etag",
    ];

    for header_name in &header_names {
        if let Some(value,) = response.headers().get(*header_name,) {
            if let Ok(value_str,) = value.to_str() {
                headers.push((header_name.to_string(), value_str.to_string(),),);
            }
        }
    }

    headers
}
