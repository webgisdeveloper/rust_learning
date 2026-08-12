use clap::Parser;
use colored::*;
use serde::Deserialize;

// Struct to deserialize the JSON response from OpenWeatherMap API
#[derive(Deserialize, Debug)]
struct WeatherResponse {
    coord: Coord,
    weather: Vec<Weather>,
    main: Main,
    wind: Wind,
    name: String,
}

// Struct to deserialize the coordinates part of the JSON response
#[derive(Deserialize, Debug)]
struct Coord {
    lon: f64,
    lat: f64,
}

// Struct to deserialize the weather part of the JSON response
#[derive(Deserialize, Debug)]
struct Weather {
    description: String,
}

// Struct to deserialize the main part of the JSON response
#[derive(Deserialize, Debug)]
struct Main {
    temp: f64,
    humidity: f64,
    pressure: f64,
}

// Struct to deserialize the wind part of the JSON response
#[derive(Deserialize, Debug)]
struct Wind {
    speed: f64,
    //deg: f64,
}

/// Simple CLI to fetch weather information from OpenWeatherMap
#[derive(Parser, Debug)]
#[command(name = "weather_cli", version, about = "Fetch current weather for a city")]
struct Args {
    /// City name (e.g., London, Tokyo, "New York")
    #[arg(value_name = "CITY")]
    city: String,

    /// Country code (ISO 3166, e.g., US, GB, JP). Optional but helps disambiguate cities.
    #[arg(value_name = "COUNTRY_CODE")]
    country_code: Option<String>,

    /// OpenWeatherMap API key (or set OPENWEATHER_API_KEY env var)
    #[arg(long, env = "OPENWEATHER_API_KEY", hide_env_values = true)]
    api_key: Option<String>,
}

// Function to get the weather data from OpenWeatherMap API
// API documentation: https://openweathermap.org/current
fn get_weather_info(
    city: &str,
    country_code: &str,
    api_key: &str,
) -> Result<WeatherResponse, reqwest::Error> {
    // Construct the URL for the API request
    // https://api.openweathermap.org/data/2.5/weather?q={city name},{country code}&appid={API key}
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

// Function to display the weather information in a user-friendly format
fn display_weather_info(weather_info: &WeatherResponse) {
    println!("Weather information for: {}", weather_info.name.green().bold());
    println!("Description: {}", weather_info.weather[0].description.yellow());
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
}

fn main() {
    let args = Args::parse();

    // Fallback to hardcoded key for backwards compatibility if neither --api-key nor env var is set
    let api_key = args.api_key.unwrap_or_else(|| "33140f2a8f0076d2cf78c6c2b2cbd08b".to_string());
    let country_code = args.country_code.unwrap_or_default();

    match get_weather_info(&args.city, &country_code, &api_key) {
        Ok(weather_info) => display_weather_info(&weather_info),
        Err(e) => {
            eprintln!("{} {}", "Error fetching weather data:".red(), e);
            std::process::exit(1);
        }
    }
}
