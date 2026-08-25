use colored::*;
use crate::models::{GeoResult, IpApiResponse, WeatherResponse};

pub fn geocode(
    city: &str,
    state: Option<&str>,
    country: Option<&str>,
    api_key: &str,
) -> Result<Vec<GeoResult>, reqwest::Error> {
    // Build q as city[,state][,country] per OWM Geocoding docs
    let mut parts = vec![city.to_string()];
    if let Some(s) = state {
        if !s.is_empty() {
            parts.push(s.to_string());
        }
    }
    if let Some(c) = country {
        if !c.is_empty() {
            parts.push(c.to_string());
        }
    }
    let q = parts.join(",");
    let url = format!(
        "http://api.openweathermap.org/geo/1.0/direct?q={}&limit=5&appid={}",
        q, api_key
    );
    println!("{} {}", "Geocoding:".blue(), url.blue().underline());
    let resp = reqwest::blocking::get(&url)?;
    let results = resp.json::<Vec<GeoResult>>()?;
    Ok(results)
}

pub fn geocode_zip(zip: &str, api_key: &str) -> Result<GeoResult, reqwest::Error> {
    // zip can be "47401" or "47401,US"
    let url = format!(
        "http://api.openweathermap.org/geo/1.0/zip?zip={}&appid={}",
        zip, api_key
    );
    println!("{} {}", "Geocoding ZIP:".blue(), url.blue().underline());
    let resp = reqwest::blocking::get(&url)?;
    let result = resp.json::<GeoResult>()?;
    Ok(result)
}

pub fn get_weather_by_coords(
    lat: f64,
    lon: f64,
    api_key: &str,
) -> Result<WeatherResponse, reqwest::Error> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric",
        lat, lon, api_key
    );
    println!("{} {}", "Fetching weather data from:".blue(), url.blue().underline());
    let response = reqwest::blocking::get(&url)?;
    let response_json = response.json::<WeatherResponse>()?;
    Ok(response_json)
}

/// Auto-detect location via IP geolocation (ip-api.com, no API key required).
/// Falls back to ipinfo.io if the primary service fails.
pub fn auto_detect_location() -> Result<GeoResult, Box<dyn std::error::Error>> {
    // Primary: ip-api.com
    let primary_url = "http://ip-api.com/json/?fields=status,message,country,countryCode,region,regionName,city,lat,lon,zip,query";
    println!("{} {}", "Auto-detecting location via IP:".blue(), primary_url.blue().underline());
    let resp = reqwest::blocking::get(primary_url)?;
    let ip: IpApiResponse = resp.json()?;
    if ip.status == "success" {
        if let (Some(lat), Some(lon)) = (ip.lat, ip.lon) {
            let city = ip.city.clone().unwrap_or_else(|| "Unknown".to_string());
            let country = ip.country_code.clone().unwrap_or_else(|| ip.country.clone().unwrap_or_default());
            let state = ip.region.clone().filter(|s| !s.is_empty()).or_else(|| ip.region_name.clone());
            println!(
                "{} {} ({}), {} [{:.4},{:.4}]",
                "Detected:".green(),
                city,
                state.clone().unwrap_or_default(),
                country,
                lat,
                lon
            );
            return Ok(GeoResult {
                name: city,
                lat,
                lon,
                country,
                state,
            });
        }
    }
    // Fallback: ipinfo.io (no token, limited rate)
    let fallback_url = "https://ipinfo.io/json";
    println!("{} {} - trying fallback...", "Primary IP geolocation failed:".yellow(), ip.message.unwrap_or_default());
    println!("{} {}", "Trying fallback:".blue(), fallback_url.blue().underline());
    let resp = reqwest::blocking::get(fallback_url)?;
    let json: serde_json::Value = resp.json()?;
    // ipinfo returns loc as "lat,lon"
    if let Some(loc_str) = json.get("loc").and_then(|v| v.as_str()) {
        let mut parts = loc_str.split(',');
        if let (Some(lat_s), Some(lon_s)) = (parts.next(), parts.next()) {
            if let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>()) {
                let city = json.get("city").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                let country = json.get("country").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let state = json.get("region").and_then(|v| v.as_str()).map(|s| s.to_string());
                println!(
                    "{} {} ({}), {} [{:.4},{:.4}]",
                    "Detected (fallback):".green(),
                    city,
                    state.clone().unwrap_or_default(),
                    country,
                    lat,
                    lon
                );
                return Ok(GeoResult {
                    name: city,
                    lat,
                    lon,
                    country,
                    state,
                });
            }
        }
    }
    Err(format!("Failed to auto-detect location: {}", json).into())
}

// Legacy direct q=city lookup (kept as fallback, but geocoding+coords is preferred for disambiguation)
pub fn get_weather_info(
    city: &str,
    country_code: &str,
    api_key: &str,
) -> Result<WeatherResponse, reqwest::Error> {
    let query = if country_code.is_empty() {
        city.to_string()
    } else {
        format!("{},{}", city, country_code)
    };
    let url = format!(
        "http://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
        query, api_key
    );
    println!("{} {}", "Fetching weather data from:".blue(), url.blue().underline());
    let response = reqwest::blocking::get(&url)?;
    let response_json = response.json::<WeatherResponse>()?;
    Ok(response_json)
}
