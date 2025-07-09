//! Tests for URL parsing and validation

use url::Url;

#[test]
fn test_url_parsing() {
    let valid_urls = vec![
        "https://example.com",
        "http://localhost:8080",
        "https://api.example.com/v1/health",
        "http://192.168.1.100:3000/status",
    ];

    for url_str in valid_urls {
        let url = Url::parse(url_str);
        assert!(url.is_ok(), "Failed to parse URL: {url_str}");
    }
}

#[test]
fn test_invalid_url_parsing() {
    let invalid_urls = vec!["not-a-url", "", "http://"];

    for url_str in invalid_urls {
        let url = Url::parse(url_str);
        assert!(url.is_err(), "Should fail to parse invalid URL: {url_str}");
    }
}

#[test]
fn test_url_schemes() {
    let test_cases = vec![
        ("https://example.com", "https"),
        ("http://localhost", "http"),
        ("ftp://example.com", "ftp"),
    ];

    for (url_str, expected_scheme) in test_cases {
        let url = Url::parse(url_str).unwrap();
        assert_eq!(url.scheme(), expected_scheme);
    }
}

#[test]
fn test_url_components() {
    let url = Url::parse("https://api.example.com:8080/v1/health?timeout=10").unwrap();

    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("api.example.com"));
    assert_eq!(url.port(), Some(8080));
    assert_eq!(url.path(), "/v1/health");
    assert_eq!(url.query(), Some("timeout=10"));
}

#[test]
fn test_url_normalization() {
    let urls = vec!["https://example.com", "https://example.com/"];

    for url_str in urls {
        let url = Url::parse(url_str).unwrap();
        // Both should normalize to the same form
        assert!(url.as_str().ends_with('/'));
    }
}

#[test]
fn test_url_with_params() {
    let url = Url::parse("https://example.com/search?q=rust&type=crate").unwrap();

    let params: Vec<_> = url.query_pairs().collect();
    assert_eq!(params.len(), 2);
    assert!(params.contains(&("q".into(), "rust".into())));
    assert!(params.contains(&("type".into(), "crate".into())));
}
