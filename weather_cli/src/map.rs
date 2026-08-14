use colored::*;
use crate::models::GeoResult;

pub const MAP_W: usize = 80;
pub const MAP_H: usize = 40;

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

pub fn render_ascii_world_map(lat: f64, lon: f64, resolved: Option<&GeoResult>) {
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

    println!("{}", format!(" World Map ({} )", header_label).cyan().bold());
    // Box is exactly 80 chars wide inside, 40 rows tall
    println!("{}", "┌────────────────────────────────────────────────────────────────────────────────┐".dimmed());
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
    println!("{}", "└────────────────────────────────────────────────────────────────────────────────┘".dimmed());
    println!("{}", "★ = location".red().dimmed());
}
