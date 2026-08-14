mod api;
mod cli;
mod display;
mod map;
mod models;

use clap::Parser;
use colored::*;

use api::{geocode, geocode_zip, get_weather_by_coords, get_weather_info};
use cli::Args;
use display::{display_weather_info, print_geo_table};

fn main() {
    let args = Args::parse();

    let api_key = args
        .api_key
        .unwrap_or_else(|| "33140f2a8f0076d2cf78c6c2b2cbd08b".to_string());

    // 1) Direct coordinate mode – most precise, no ambiguity
    if let (Some(lat), Some(lon)) = (args.lat, args.lon) {
        match get_weather_by_coords(lat, lon, &api_key) {
            Ok(info) => display_weather_info(&info, None, args.map),
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
                    Ok(info) => display_weather_info(&info, Some(&loc), args.map),
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
            eprintln!(
                "{}",
                "error: CITY is required unless --lat/--lon or --zip is used".red()
            );
            eprintln!(
                "Usage: weather_cli <CITY> [COUNTRY_CODE] [--state STATE] [--country COUNTRY]"
            );
            eprintln!("Try 'weather_cli --help' for more information.");
            std::process::exit(2);
        }
    };

    // Resolve country: explicit --country wins over positional
    let country = args.country.or(args.country_code);
    let state = args.state;

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

    // De-duplicate / disambiguate: if multiple results but only one has exact name match, treat as unambiguous
    // e.g. "London" -> [London, City of London] should auto-pick London
    let mut effective_results = geo_results;
    if effective_results.len() > 1 && !args.all {
        let exact_matches: Vec<_> = effective_results
            .iter()
            .filter(|r| r.name.eq_ignore_ascii_case(&city))
            .cloned()
            .collect();
        if exact_matches.len() == 1 {
            effective_results = exact_matches;
        } else if effective_results.len() > 1 {
            // If all results are clustered within ~0.5° (~30km), treat as same city
            let (min_lat, max_lat) = effective_results.iter().fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(mn, mx), r| (mn.min(r.lat), mx.max(r.lat)),
            );
            let (min_lon, max_lon) = effective_results.iter().fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(mn, mx), r| (mn.min(r.lon), mx.max(r.lon)),
            );
            if (max_lat - min_lat) < 0.5 && (max_lon - min_lon) < 0.5 {
                effective_results = vec![effective_results[0].clone()];
            } else if exact_matches.len() > 1 && exact_matches.len() < effective_results.len() {
                let (emin_lat, emax_lat) = exact_matches.iter().fold(
                    (f64::INFINITY, f64::NEG_INFINITY),
                    |(mn, mx), r| (mn.min(r.lat), mx.max(r.lat)),
                );
                let (emin_lon, emax_lon) = exact_matches.iter().fold(
                    (f64::INFINITY, f64::NEG_INFINITY),
                    |(mn, mx), r| (mn.min(r.lon), mx.max(r.lon)),
                );
                if (emax_lat - emin_lat) < 0.5 && (emax_lon - emin_lon) < 0.5 {
                    effective_results = vec![exact_matches[0].clone()];
                } else {
                    effective_results = exact_matches;
                }
            }
        }
    }

    if effective_results.len() > 1 && !args.all {
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
    let targets: Vec<&_> = if args.all {
        effective_results.iter().collect()
    } else {
        vec![&effective_results[0]]
    };

    for loc in targets {
        match get_weather_by_coords(loc.lat, loc.lon, &api_key) {
            Ok(info) => {
                display_weather_info(&info, Some(loc), args.map);
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
                let cc = loc.country.clone();
                if let Ok(fb) = get_weather_info(&loc.name, &cc, &api_key) {
                    display_weather_info(&fb, Some(loc), args.map);
                }
            }
        }
    }
}
