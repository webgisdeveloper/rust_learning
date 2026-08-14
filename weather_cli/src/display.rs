use colored::*;
use crate::map::render_ascii_world_map;
use crate::models::{GeoResult, WeatherResponse};

pub fn display_weather_info(
    weather_info: &WeatherResponse,
    resolved: Option<&GeoResult>,
    show_map: bool,
) {
    if let Some(loc) = resolved {
        let state_part = loc
            .state
            .as_deref()
            .map(|s| format!(", {}", s))
            .unwrap_or_default();
        println!(
            "Weather information for: {}{}, {} {}",
            loc.name.green().bold(),
            state_part.green().bold(),
            loc.country.green().bold(),
            format!("({:.4}, {:.4})", loc.lat, loc.lon).dimmed()
        );
        if loc.name != weather_info.name {
            println!("  (API reports city name: {})", weather_info.name);
        }
    } else {
        println!(
            "Weather information for: {}",
            weather_info.name.green().bold()
        );
    }
    println!(
        "Description: {}",
        weather_info.weather[0].description.yellow()
    );
    println!(
        "Temperature: {} °C",
        format!("{:.1}", weather_info.main.temp).cyan()
    );
    println!(
        "Humidity: {} %",
        format!("{:.0}", weather_info.main.humidity).cyan()
    );
    println!(
        "Pressure: {} hPa",
        format!("{:.0}", weather_info.main.pressure).cyan()
    );
    println!(
        "Wind Speed: {} m/s",
        format!("{:.1}", weather_info.wind.speed).cyan()
    );
    println!(
        "Coordinates: Latitude: {}, Longitude: {}",
        weather_info.coord.lat, weather_info.coord.lon
    );

    if show_map {
        let lat = resolved.map(|r| r.lat).unwrap_or(weather_info.coord.lat);
        let lon = resolved.map(|r| r.lon).unwrap_or(weather_info.coord.lon);
        println!();
        render_ascii_world_map(lat, lon, resolved);
    }
}

pub fn print_geo_table(results: &[GeoResult]) {
    println!("{}", "Matching locations:".green().bold());
    for (i, r) in results.iter().enumerate() {
        let state = r.state.as_deref().unwrap_or("-");
        println!(
            "  {}. {} | state: {:<4} | country: {} | lat: {:.4}, lon: {:.4}",
            i + 1,
            r.name.bold(),
            state,
            r.country,
            r.lat,
            r.lon
        );
    }
    println!(
        "\n{}",
        "Hint: re-run with --state and/or --country to pick one, e.g.:".yellow()
    );
    if let Some(first) = results.first() {
        for r in results.iter().take(2) {
            let s = r.state.clone().unwrap_or_default();
            if !s.is_empty() {
                println!(
                    "  weather_cli {} --state {} --country {}",
                    first.name, s, r.country
                );
            } else {
                println!("  weather_cli {} --country {}", r.name, r.country);
            }
        }
    }
    println!("Or use exact coordinates: weather_cli --lat <LAT> --lon <LON>");
    println!("Or use ZIP: weather_cli --zip 47401,US");
}
