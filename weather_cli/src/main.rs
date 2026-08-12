use clap::Parser;
use colored::*;
use serde::Deserialize;

// ---------- OpenWeatherMap response structs ----------

#[derive(Deserialize, Debug)]
struct WeatherResponse {
    coord: Coord,
    weather: Vec<Weather>,
    main: Main,
    wind: Wind,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Coord {
    lon: f64,
    lat: f64,
}

#[derive(Deserialize, Debug)]
struct Weather {
    description: String,
}

#[derive(Deserialize, Debug)]
struct Main {
    temp: f64,
    humidity: f64,
    pressure: f64,
}

#[derive(Deserialize, Debug)]
struct Wind {
    speed: f64,
}

// Geocoding API response: http://api.openweathermap.org/geo/1.0/direct
#[derive(Deserialize, Debug, Clone)]
struct GeoResult {
    name: String,
    lat: f64,
    lon: f64,
    country: String,
    state: Option<String>,
}

// ---------- CLI args ----------

/// Fetch current weather. Handles duplicate city names via --state / --country or --lat/--lon.
#[derive(Parser, Debug)]
#[command(name = "weather_cli", version, about = "Fetch current weather for a city")]
struct Args {
    /// City name (e.g., London, Tokyo, "New York", Bloomington).
    /// For disambiguation you can also quote "Bloomington, IN" but prefer --state/--country.
    #[arg(value_name = "CITY")]
    city: Option<String>,

    /// Country code (ISO 3166, e.g., US, GB, JP). Positional shorthand for --country.
    #[arg(value_name = "COUNTRY_CODE")]
    country_code: Option<String>,

    /// State / region code (e.g., IN, IL, CA, TX). Crucial for disambiguating same-name cities in the US.
    /// Example: --state IN for Bloomington, Indiana vs --state IL for Bloomington, Illinois.
    #[arg(long, value_name = "STATE")]
    state: Option<String>,

    /// Country code as explicit flag (takes precedence over positional COUNTRY_CODE).
    #[arg(long, value_name = "COUNTRY")]
    country: Option<String>,

    /// Latitude (requires --lon). When given, bypasses name lookup entirely – the most precise way.
    #[arg(long, value_name = "LAT", requires = "lon", allow_hyphen_values = true)]
    lat: Option<f64>,

    /// Longitude (requires --lat).
    #[arg(long, value_name = "LON", requires = "lat", allow_hyphen_values = true)]
    lon: Option<f64>,

    /// ZIP / postal code (e.g., 47401,US). Alternative to city/state lookup via zip.
    #[arg(long, value_name = "ZIP")]
    zip: Option<String>,

    /// Only list matching locations (via Geocoding API) without fetching weather. Useful to discover --state values.
    #[arg(long)]
    list: bool,

    /// Show weather for all matched locations (instead of erroring on ambiguous names).
    #[arg(long)]
    all: bool,

    /// OpenWeatherMap API key (or set OPENWEATHER_API_KEY env var)
    #[arg(long, env = "OPENWEATHER_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    /// Disable ASCII world map rendering
    #[arg(long)]
    no_map: bool,
}

// ---------- API helpers ----------

fn geocode(
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

fn geocode_zip(zip: &str, api_key: &str) -> Result<GeoResult, reqwest::Error> {
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

fn get_weather_by_coords(lat: f64, lon: f64, api_key: &str) -> Result<WeatherResponse, reqwest::Error> {
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
fn get_weather_info(city: &str, country_code: &str, api_key: &str) -> Result<WeatherResponse, reqwest::Error> {
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

fn display_weather_info(weather_info: &WeatherResponse, resolved: Option<&GeoResult>, show_map: bool) {
    if let Some(loc) = resolved {
        let state_part = loc.state.as_deref().map(|s| format!(", {}", s)).unwrap_or_default();
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
        println!("Weather information for: {}", weather_info.name.green().bold());
    }
    println!("Description: {}", weather_info.weather[0].description.yellow());
    println!("Temperature: {} °C", format!("{:.1}", weather_info.main.temp).cyan());
    println!("Humidity: {} %", format!("{:.0}", weather_info.main.humidity).cyan());
    println!("Pressure: {} hPa", format!("{:.0}", weather_info.main.pressure).cyan());
    println!("Wind Speed: {} m/s", format!("{:.1}", weather_info.wind.speed).cyan());
    println!(
        "Coordinates: Latitude: {}, Longitude: {}",
        weather_info.coord.lat, weather_info.coord.lon
    );

    if show_map {
        let lat = resolved.map(|r| r.lat).unwrap_or(weather_info.coord.lat);
        let lon = resolved.map(|r| r.lon).unwrap_or(weather_info.coord.lon);
        println!();
        render_ascii_world_map(lat, lon, resolved.or(None));
    }
}

// ---------- ASCII World Map ----------

const MAP_W: usize = 80;
const MAP_H: usize = 40;

#[derive(Clone, Copy)]
struct Rect {
    lat_min: f64,
    lat_max: f64,
    lon_min: f64,
    lon_max: f64,
}

fn is_land(lat: f64, lon: f64) -> bool {
    // Approximate continent polygons as union of rectangles in lat/lon space.
    // Rects are chosen to keep oceans (esp. Indian/Atlantic) as water.
    const LAND_RECTS: &[Rect] = &[
        // Greenland
        Rect { lat_min: 60.0, lat_max: 83.0, lon_min: -55.0, lon_max: -12.0 },
        // North America
        Rect { lat_min: 60.0, lat_max: 71.0, lon_min: -170.0, lon_max: -145.0 }, // Alaska
        Rect { lat_min: 50.0, lat_max: 65.0, lon_min: -130.0, lon_max: -100.0 }, // Canada west
        Rect { lat_min: 50.0, lat_max: 60.0, lon_min: -100.0, lon_max: -60.0 },  // Canada east
        Rect { lat_min: 30.0, lat_max: 50.0, lon_min: -125.0, lon_max: -100.0 }, // USA west
        Rect { lat_min: 30.0, lat_max: 50.0, lon_min: -100.0, lon_max: -65.0 },  // USA east
        Rect { lat_min: 15.0, lat_max: 30.0, lon_min: -110.0, lon_max: -85.0 },  // Mexico
        Rect { lat_min: 8.0, lat_max: 15.0, lon_min: -90.0, lon_max: -75.0 },    // Central America
        // South America (tapered)
        Rect { lat_min: 0.0, lat_max: 12.0, lon_min: -80.0, lon_max: -45.0 },
        Rect { lat_min: -25.0, lat_max: 0.0, lon_min: -70.0, lon_max: -35.0 },
        Rect { lat_min: -55.0, lat_max: -25.0, lon_min: -70.0, lon_max: -55.0 },
        // Europe
        Rect { lat_min: 55.0, lat_max: 71.0, lon_min: -25.0, lon_max: 40.0 },
        Rect { lat_min: 35.0, lat_max: 55.0, lon_min: -10.0, lon_max: 40.0 },
        // Africa
        Rect { lat_min: 20.0, lat_max: 37.0, lon_min: -20.0, lon_max: 50.0 },
        Rect { lat_min: 0.0, lat_max: 20.0, lon_min: -20.0, lon_max: 45.0 },
        Rect { lat_min: -35.0, lat_max: 0.0, lon_min: 10.0, lon_max: 35.0 },
        // Middle East / Arabia
        Rect { lat_min: 12.0, lat_max: 30.0, lon_min: 35.0, lon_max: 60.0 },
        // Asia
        Rect { lat_min: 55.0, lat_max: 80.0, lon_min: 40.0, lon_max: 180.0 }, // Siberia
        Rect { lat_min: 35.0, lat_max: 55.0, lon_min: 50.0, lon_max: 140.0 }, // Central Asia
        Rect { lat_min: 8.0, lat_max: 30.0, lon_min: 68.0, lon_max: 88.0 },   // India
        Rect { lat_min: 20.0, lat_max: 45.0, lon_min: 100.0, lon_max: 125.0 }, // China
        Rect { lat_min: 30.0, lat_max: 45.0, lon_min: 130.0, lon_max: 145.0 }, // Japan
        Rect { lat_min: 10.0, lat_max: 25.0, lon_min: 90.0, lon_max: 125.0 }, // SE Asia mainland
        Rect { lat_min: -10.0, lat_max: 10.0, lon_min: 95.0, lon_max: 135.0 }, // Indonesia/Malaysia
        // Australia
        Rect { lat_min: -28.0, lat_max: -10.0, lon_min: 113.0, lon_max: 155.0 },
        Rect { lat_min: -44.0, lat_max: -38.0, lon_min: 145.0, lon_max: 150.0 }, // Tasmania
        // NZ
        Rect { lat_min: -47.0, lat_max: -34.0, lon_min: 166.0, lon_max: 179.0 },
    ];
    for r in LAND_RECTS {
        if lat >= r.lat_min && lat <= r.lat_max && lon >= r.lon_min && lon <= r.lon_max {
            return true;
        }
    }
    false
}

fn render_ascii_world_map(lat: f64, lon: f64, resolved: Option<&GeoResult>) {
    // Compute marker position in equirectangular projection
    let marker_x = (((lon + 180.0) / 360.0) * MAP_W as f64).floor() as isize;
    let marker_y = (((90.0 - lat) / 180.0) * MAP_H as f64).floor() as isize;
    let mx = marker_x.clamp(0, MAP_W as isize - 1) as usize;
    let my = marker_y.clamp(0, MAP_H as isize - 1) as usize;

    // Header without coordinate numbers - only location name
    let header_label = if let Some(r) = resolved {
        if let Some(s) = &r.state {
            format!("{} , {}", r.name, s)
        } else {
            format!("{} , {}", r.name, r.country)
        }
    } else {
        "Location".to_string()
    };

    println!(
        "{}",
        format!(" World Map ({} )", header_label).cyan().bold()
    );
    // Box is exactly 80 chars wide inside, 40 rows tall
    println!(
        "{}",
        "┌────────────────────────────────────────────────────────────────────────────────┐".dimmed()
    );
    for y in 0..MAP_H {
        let mut row = String::with_capacity(MAP_W);
        let row_lat = 90.0 - (y as f64 + 0.5) * 180.0 / MAP_H as f64;
        for x in 0..MAP_W {
            if x == mx && y == my {
                row.push('★');
                continue;
            }
            let col_lon = -180.0 + (x as f64 + 0.5) * 360.0 / MAP_W as f64;
            if is_land(row_lat, col_lon) {
                row.push('*');
            } else {
                row.push('.');
            }
        }
        // Colorize: land green, ocean blue, marker red
        let mut colored_row = String::new();
        for (x, ch) in row.chars().enumerate() {
            if x == mx && y == my {
                colored_row.push_str(&"★".red().bold().to_string());
            } else if ch == '*' {
                colored_row.push_str(&"*".green().dimmed().to_string());
            } else {
                colored_row.push_str(&".".blue().dimmed().to_string());
            }
        }
        println!("│{}│", colored_row);
    }
    println!(
        "{}",
        "└────────────────────────────────────────────────────────────────────────────────┘".dimmed()
    );
    println!("{}", "★ = location".red().dimmed());
}

fn print_geo_table(results: &[GeoResult]) {
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
    println!("\n{}", "Hint: re-run with --state and/or --country to pick one, e.g.:".yellow());
    if let Some(first) = results.first() {
        for r in results.iter().take(2) {
            let s = r.state.clone().unwrap_or_default();
            if !s.is_empty() {
                println!("  weather_cli {} --state {} --country {}", first.name, s, r.country);
            } else {
                println!("  weather_cli {} --country {}", r.name, r.country);
            }
        }
    }
    println!("Or use exact coordinates: weather_cli --lat <LAT> --lon <LON>");
    println!("Or use ZIP: weather_cli --zip 47401,US");
}

fn main() {
    let args = Args::parse();

    let api_key = args.api_key.unwrap_or_else(|| "33140f2a8f0076d2cf78c6c2b2cbd08b".to_string());

    // 1) Direct coordinate mode – most precise, no ambiguity
    if let (Some(lat), Some(lon)) = (args.lat, args.lon) {
        match get_weather_by_coords(lat, lon, &api_key) {
            Ok(info) => display_weather_info(&info, None, !args.no_map),
            Err(e) => {
                eprintln!("{} {}", "Error fetching weather data:".red(), e);
                std::process::exit(1);
            }
        }
        return;
    }

    // 2) ZIP mode
    if let Some(zip) = args.zip {
        match geocode_zip(&zip, &api_key) {
            Ok(loc) => {
                if args.list {
                    print_geo_table(std::slice::from_ref(&loc));
                    return;
                }
                match get_weather_by_coords(loc.lat, loc.lon, &api_key) {
                    Ok(info) => display_weather_info(&info, Some(&loc), !args.no_map),
                    Err(e) => {
                        eprintln!("{} {}", "Error fetching weather data:".red(), e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("{} {}", "Error geocoding ZIP:".red(), e);
                std::process::exit(1);
            }
        }
        return;
    }

    // 3) Name-based mode (city required)
    let city = match args.city {
        Some(c) => c,
        None => {
            eprintln!("{}", "error: CITY is required unless --lat/--lon or --zip is used".red());
            eprintln!("Usage: weather_cli <CITY> [COUNTRY_CODE] [--state STATE] [--country COUNTRY]");
            eprintln!("Try 'weather_cli --help' for more information.");
            std::process::exit(2);
        }
    };

    // Resolve country: explicit --country wins over positional
    let country = args.country.or(args.country_code);
    let state = args.state;

    // If user passed "Bloomington, IN" as city without --state, try to handle it naturally:
    // We keep city as-is; OWM geocoding understands comma-separated q, so "Bloomington, IN, US" works.
    // But to keep logic clean, if city already contains commas and no state/country flags, just use legacy q lookup
    // and also offer geocoding path. We'll prefer geocoding path always for disambiguation.

    let geo_results = match geocode(&city, state.as_deref(), country.as_deref(), &api_key) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} {}", "Error during geocoding:".red(), e);
            std::process::exit(1);
        }
    };

    if geo_results.is_empty() {
        eprintln!(
            "{} No locations found for '{}'{} chiefly. Try a different spelling or add --country.",
            "Error:".red(),
            city,
            country
                .as_deref()
                .map(|c| format!(" in {}", c))
                .unwrap_or_default()
        );
        std::process::exit(1);
    }

    if args.list {
        print_geo_table(&geo_results);
        return;
    }

    // De-duplicate / disambiguate: if multiple results but only one has exact name match (case-insensitive), treat as unambiguous
    // e.g. "London" -> [London, City of London] should auto-pick London
    let mut effective_results = geo_results;
    if effective_results.len() > 1 && !args.all {
        let exact_matches: Vec<GeoResult> = effective_results
            .iter()
            .filter(|r| r.name.eq_ignore_ascii_case(&city))
            .cloned()
            .collect();
        if exact_matches.len() == 1 {
            effective_results = exact_matches;
        } else if effective_results.len() > 1 {
            // If all results are clustered within ~0.3° (~30km), treat as same city (different OSM subdivisions)
            // e.g. Sydney, AU returns 4 entries all within -33.88±0.02 lat/lon – pick first
            let (min_lat, max_lat) = effective_results
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), r| (mn.min(r.lat), mx.max(r.lat)));
            let (min_lon, max_lon) = effective_results
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), r| (mn.min(r.lon), mx.max(r.lon)));
            if (max_lat - min_lat) < 0.5 && (max_lon - min_lon) < 0.5 {
                effective_results = vec![effective_results[0].clone()];
            } else if exact_matches.len() > 1 && exact_matches.len() < effective_results.len() {
                // Prefer exact name matches when clustered differently, but still check spread inside exact_matches
                let (emin_lat, emax_lat) = exact_matches
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), r| (mn.min(r.lat), mx.max(r.lat)));
                let (emin_lon, emax_lon) = exact_matches
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), r| (mn.min(r.lon), mx.max(r.lon)));
                if (emax_lat - emin_lat) < 0.5 && (emax_lon - emin_lon) < 0.5 {
                    effective_results = vec![exact_matches[0].clone()];
                } else {
                    // Keep only exact matches for clearer ambiguous table
                    effective_results = exact_matches;
                }
            }
        }
    }

    if effective_results.len() > 1 && !args.all {
        // Ambiguous – require disambiguation
        eprintln!(
            "{} Found {} locations matching '{}'{}:",
            "Ambiguous:".yellow().bold(),
            effective_results.len(),
            city,
            country
                .as_deref()
                .map(|c| format!(" in {}", c))
                .unwrap_or_default()
        );
        print_geo_table(&effective_results);
        eprintln!(
            "\n{} Use --state/--country to disambiguate or --all to fetch all, --list to only list.",
            "Tip:".cyan()
        );
        std::process::exit(1);
    }

    // Fetch weather for one or all matches
    let targets: Vec<&GeoResult> = if args.all {
        effective_results.iter().collect()
    } else {
        vec![&effective_results[0]]
    };

    for loc in targets {
        match get_weather_by_coords(loc.lat, loc.lon, &api_key) {
            Ok(info) => {
                display_weather_info(&info, Some(loc), !args.no_map);
                if args.all {
                    println!("{}", "---".dimmed());
                }
            }
            Err(e) => {
                eprintln!(
                    "{} Failed for {},{} ({:.4},{:.4}): {}",
                    "Error:".red(),
                    loc.name,
                    loc.state.as_deref().unwrap_or(&loc.country),
                    loc.lat,
                    loc.lon,
                    e
                );
                // Fallback to legacy q lookup for this loc
                let cc = loc.country.clone();
                if let Ok(fb) = get_weather_info(&loc.name, &cc, &api_key) {
                    display_weather_info(&fb, Some(loc), !args.no_map);
                }
            }
        }
    }
}
