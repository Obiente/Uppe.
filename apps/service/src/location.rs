use crate::config::LocationPrivacy;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

static LOCATION_CACHE: OnceLock<Arc<RwLock<LocationCache>>> = OnceLock::new();

struct LocationCache {
    location: Location,
    privacy_level: LocationPrivacy,
    last_update: Instant,
    update_interval: Duration,
}

/// Response from ip-api.com geolocation service
#[derive(Debug, Deserialize)]
struct IpApiResponse {
    #[serde(default)]
    city: String,
    #[serde(rename = "countryCode", default)]
    country_code: String,
    #[serde(default)]
    status: String,
}

/// Geographic location information (general, privacy-preserving)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    /// City name (e.g., "New York", "London")
    pub city: Option<String>,
    /// ISO 3166-1 alpha-2 country code (e.g., "US", "GB", "JP")
    pub country: Option<String>,
    /// General region/continent (e.g., "North America", "Europe", "Asia")
    pub region: Option<String>,
}

impl Location {
    pub fn new(city: Option<String>, country: Option<String>, region: Option<String>) -> Self {
        Self { city, country, region }
    }

    /// Create unknown/unconfigured location
    pub fn unknown() -> Self {
        Self { city: None, country: None, region: Some("Unknown".to_string()) }
    }

    /// Apply privacy level to location, removing sensitive data
    pub fn apply_privacy(&self, privacy: LocationPrivacy) -> Self {
        match privacy {
            LocationPrivacy::Disabled => Location::unknown(),
            LocationPrivacy::CountryOnly => {
                Location::new(None, self.country.clone(), self.region.clone())
            }
            LocationPrivacy::Full => self.clone(),
        }
    }

    /// Format location for display
    pub fn display(&self) -> String {
        let mut parts = Vec::new();

        if let Some(city) = &self.city {
            parts.push(city.clone());
        }

        if let Some(country) = &self.country {
            parts.push(country.clone());
        }

        if parts.is_empty() {
            if let Some(region) = &self.region {
                return region.clone();
            }
            return "Unknown".to_string();
        }

        parts.join(", ")
    }

    /// Get region from country code (simplified mapping)
    pub fn region_from_country(country_code: &str) -> &'static str {
        match country_code {
            // North America
            "US" | "CA" | "MX" => "North America",

            // Europe
            "GB" | "FR" | "DE" | "IT" | "ES" | "NL" | "BE" | "CH" | "AT" | "SE" | "NO" | "DK"
            | "FI" | "PL" | "CZ" | "PT" | "GR" | "IE" | "HU" | "RO" | "UA" => "Europe",

            // Asia
            "CN" | "JP" | "KR" | "IN" | "SG" | "HK" | "TW" | "TH" | "MY" | "ID" | "PH" | "VN" => {
                "Asia"
            }

            // South America
            "BR" | "AR" | "CL" | "CO" | "PE" | "VE" | "EC" | "UY" => "South America",

            // Oceania
            "AU" | "NZ" => "Oceania",

            // Middle East
            "AE" | "SA" | "IL" | "TR" | "IR" | "IQ" | "JO" | "KW" | "QA" | "BH" | "OM" => {
                "Middle East"
            }

            // Africa
            "ZA" | "EG" | "NG" | "KE" | "MA" | "GH" | "ET" | "TZ" | "UG" => "Africa",

            _ => "Other",
        }
    }
}

/// Fetch location from IP geolocation API
async fn fetch_location_from_ip() -> Result<Location> {
    // Use ip-api.com - free, no API key required, 45 requests/minute
    let response =
        reqwest::get("http://ip-api.com/json/?fields=status,city,countryCode,regionName")
            .await?
            .json::<IpApiResponse>()
            .await?;

    if response.status != "success" {
        return Ok(Location::unknown());
    }

    let city = if !response.city.is_empty() { Some(response.city) } else { None };

    let country =
        if !response.country_code.is_empty() { Some(response.country_code.clone()) } else { None };

    let region = country.as_ref().map(|cc| Location::region_from_country(cc).to_string());

    Ok(Location::new(city, country, region))
}

/// Initialize location cache with update interval (in seconds) and privacy level
pub fn init_location_cache(update_interval_secs: u64, privacy_level: LocationPrivacy) {
    let cache = LocationCache {
        location: Location::unknown(),
        privacy_level,
        last_update: Instant::now() - Duration::from_secs(update_interval_secs + 1), /* Force immediate update */
        update_interval: Duration::from_secs(update_interval_secs),
    };

    let _ = LOCATION_CACHE.set(Arc::new(RwLock::new(cache)));
}

/// Initialize location from static config (for backwards compatibility)
pub fn init_location(location: Location) {
    init_location_cache(0, LocationPrivacy::Full); // 0 = never auto-update
    if let Some(cache) = LOCATION_CACHE.get()
        && let Ok(mut cache) = cache.write() {
            cache.location = location;
            cache.last_update = Instant::now();
        }
}

/// Get the configured location (with automatic updates if enabled)
pub fn get_location() -> Location {
    let cache = LOCATION_CACHE.get_or_init(|| {
        Arc::new(RwLock::new(LocationCache {
            location: Location::unknown(),
            privacy_level: LocationPrivacy::Full,
            last_update: Instant::now(),
            update_interval: Duration::from_secs(0),
        }))
    });

    // Try to read current location
    if let Ok(cache_guard) = cache.read() {
        return cache_guard.location.clone();
    }

    Location::unknown()
}

/// Update location from IP (non-blocking, spawns background task)
pub fn update_location_from_ip() {
    if let Some(cache) = LOCATION_CACHE.get() {
        let cache_clone = Arc::clone(cache);
        tokio::spawn(async move {
            match fetch_location_from_ip().await {
                Ok(new_location) => {
                    if let Ok(mut cache) = cache_clone.write() {
                        // Apply privacy level before storing
                        cache.location = new_location.apply_privacy(cache.privacy_level);
                        cache.last_update = Instant::now();
                        tracing::info!("Location updated: {}", cache.location.display());
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to update location from IP: {}", e);
                }
            }
        });
    }
}

/// Check if location needs update and update if necessary
pub fn check_and_update_location() {
    if let Some(cache) = LOCATION_CACHE.get() {
        let needs_update = if let Ok(cache_guard) = cache.read() {
            cache_guard.update_interval.as_secs() > 0
                && cache_guard.last_update.elapsed() >= cache_guard.update_interval
        } else {
            false
        };

        if needs_update {
            update_location_from_ip();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_display() {
        let loc = Location::new(
            Some("New York".to_string()),
            Some("US".to_string()),
            Some("North America".to_string()),
        );
        assert_eq!(loc.display(), "New York, US");
    }

    #[test]
    fn test_unknown_location() {
        let loc = Location::unknown();
        assert_eq!(loc.display(), "Unknown");
    }

    #[test]
    fn test_region_mapping() {
        assert_eq!(Location::region_from_country("US"), "North America");
        assert_eq!(Location::region_from_country("GB"), "Europe");
        assert_eq!(Location::region_from_country("JP"), "Asia");
        assert_eq!(Location::region_from_country("AU"), "Oceania");
        assert_eq!(Location::region_from_country("ZZ"), "Other");
    }
}
