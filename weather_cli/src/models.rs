use serde::Deserialize;

// ---------- OpenWeatherMap weather response ----------

#[derive(Deserialize, Debug)]
pub struct WeatherResponse {
    pub coord: Coord,
    pub weather: Vec<Weather>,
    pub main: Main,
    pub wind: Wind,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Coord {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Deserialize, Debug)]
pub struct Weather {
    pub description: String,
}

#[derive(Deserialize, Debug)]
pub struct Main {
    pub temp: f64,
    pub humidity: f64,
    pub pressure: f64,
}

#[derive(Deserialize, Debug)]
pub struct Wind {
    pub speed: f64,
}

// Geocoding API response: http://api.openweathermap.org/geo/1.0/direct
#[derive(Deserialize, Debug, Clone)]
pub struct GeoResult {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub country: String,
    pub state: Option<String>,
}
