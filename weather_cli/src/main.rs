use std::io;
use serde::Deserialize;
use colored::*;

//Struct to desieralize the JSON response from openWeatherMap API
#[derive(Deserialize, Debug)]
struct WeatherResponse {
    coord: Coord,
    weather: Vec<Weather>,
    main: Main,
    wind: Wind,
    name: String,
}

//Struct to desieralize the coordinates part of the JSON response
#[derive(Deserialize, Debug)]
struct Coord {
    lon: f64,
    lat: f64,
}

//Struct to desieralize the weather part of the JSON response
#[derive(Deserialize, Debug)]
struct Weather {
    description: String,
}

//Struct to desieralize the main part of the JSON response
#[derive(Deserialize, Debug)]
struct Main {
    temp: f64,
    humidity: f64,
    pressure: f64,
}

//Struct to desieralize the wind part of the JSON response
#[derive(Deserialize, Debug)]
struct Wind {
    speed: f64,
    //deg: f64,
}

//Function to get the weather data from openWeatherMap API
//API documentation: https://openweathermap.org/current
fn get_weather_info(city: &str, country_code: &str, api_key: &str) -> Result<WeatherResponse, reqwest::Error> {
    //Construct the URL for the API request
    //https://api.openweathermap.org/data/2.5/weather?q={city name},{country code}&appid={API key}
    
    /* The format! macro is Rust's primary tool for creating formatted strings. 
    It works similarly to println! and print!, 
    it returns a String that you can store in a variable or use elsewhere in your code.
    */
    let url = format!(
        "http://api.openweathermap.org/data/2.5/weather?q={},{}&appid={}&units=metric",
        city, country_code, api_key
    );
    
    println!("{} {}", "Fetching weather data from:".blue(), url.blue().underline());
    /* Rust allows you to access any public item using its full path: crate_name::module::item
    You can also bring items into scope with the use keyword to avoid repeating the full path.
    use reqwest::blocking; 
    let response = blocking::get(&url)?;
    */
    /* The ? operator in Rust is a powerful shorthand for error propagation, often called the "try operator."
    When you append ? to a Result or Option type, it does the following:
    1. If the value is Ok or Some, it unwraps the value and allows the program to continue.
    2. If the value is Err or None, it returns the error or None from the current function, effectively propagating the error up the call stack.
    */
    // Sending a blocking GET request to the API endpoint
    let response = reqwest::blocking::get(&url)?;
    // Parsing the JSON response into WeatherResponse struct
    let response_json = response.json::<WeatherResponse>()?;
    Ok(response_json) // Returning the deserialized response
}

//funcation to display the weather information in a user-friendly format
fn display_weather_info(weather_info: &WeatherResponse) {
    println!("Weather information for: {}", weather_info.name.green().bold());
    println!("Description: {}", weather_info.weather[0].description.yellow());
    println!("Temperature: {} °C", format!("{:.1}", weather_info.main.temp).cyan());
    println!("Humidity: {} %", format!("{:.0}", weather_info.main.humidity).cyan());
    println!("Pressure: {} hPa", format!("{:.0}", weather_info.main.pressure).cyan());
    println!("Wind Speed: {} m/s", format!("{:.1}", weather_info.wind.speed).cyan());
    println!("Coordinates: Latitude: {}, Longitude: {}", weather_info.coord.lat, weather_info.coord.lon);
}

fn main() {
    println!("{}", "Welcome to the Weather CLI!".blue().bold());
    let api_key ="33140f2a8f0076d2cf78c6c2b2cbd08b";
    loop {
        println!("{}", "Enter city name (or type 'exit' to quit):".blue());
        let mut city = String::new();
        io::stdin().read_line(&mut city).expect("Failed to read line");
        let city = city.trim();
        if city.eq_ignore_ascii_case("exit") {
            println!("{}", "Exiting Weather CLI. Goodbye!".blue().bold());
            break;
        }
        println!("{}", "Enter country code (e.g., US for United States):".blue());
        let mut country_code = String::new();
        io::stdin().read_line(&mut country_code).expect("Failed to read line");
        let country_code = country_code.trim();

        match get_weather_info(city, country_code, api_key) {
            Ok(weather_info) => {display_weather_info(&weather_info)},
            Err(e) => {println!("{} {}", "Error fetching weather data:".red(), e)},
        }
    }
}
