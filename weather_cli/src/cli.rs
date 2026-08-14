use clap::Parser;

/// Fetch current weather. Handles duplicate city names via --state / --country or --lat/--lon.
#[derive(Parser, Debug)]
#[command(name = "weather_cli", version, about = "Fetch current weather for a city")]
pub struct Args {
    /// City name (e.g., London, Tokyo, "New York", Bloomington).
    /// For disambiguation you can also quote "Bloomington, IN" but prefer --state/--country.
    #[arg(value_name = "CITY")]
    pub city: Option<String>,

    /// Country code (ISO 3166, e.g., US, GB, JP). Positional shorthand for --country.
    #[arg(value_name = "COUNTRY_CODE")]
    pub country_code: Option<String>,

    /// State / region code (e.g., IN, IL, CA, TX). Crucial for disambiguating same-name cities in the US.
    /// Example: --state IN for Bloomington, Indiana vs --state IL for Bloomington, Illinois.
    #[arg(long, value_name = "STATE")]
    pub state: Option<String>,

    /// Country code as explicit flag (takes precedence over positional COUNTRY_CODE).
    #[arg(long, value_name = "COUNTRY")]
    pub country: Option<String>,

    /// Latitude (requires --lon). When given, bypasses name lookup entirely – the most precise way.
    #[arg(long, value_name = "LAT", requires = "lon", allow_hyphen_values = true)]
    pub lat: Option<f64>,

    /// Longitude (requires --lat).
    #[arg(long, value_name = "LON", requires = "lat", allow_hyphen_values = true)]
    pub lon: Option<f64>,

    /// ZIP / postal code (e.g., 47401,US). Alternative to city/state lookup via zip.
    #[arg(long, value_name = "ZIP")]
    pub zip: Option<String>,

    /// Only list matching locations (via Geocoding API) without fetching weather. Useful to discover --state values.
    #[arg(long)]
    pub list: bool,

    /// Show weather for all matched locations (instead of erroring on ambiguous names).
    #[arg(long)]
    pub all: bool,

    /// OpenWeatherMap API key (or set OPENWEATHER_API_KEY env var)
    #[arg(long, env = "OPENWEATHER_API_KEY", hide_env_values = true)]
    pub api_key: Option<String>,

    /// Show ASCII world map (80×40 box, * for land, . for sea, ★ marks location) – off by default
    #[arg(long)]
    pub map: bool,
}
