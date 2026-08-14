use colored::*;
use crate::models::{GeoResult, WeatherResponse};

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
