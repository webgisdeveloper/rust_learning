use crate::models::GeoResult;
use colored::*;
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::Rect,
    style::{Color, Style},
    widgets::{
        Block, Borders,
        canvas::{Canvas, Map, MapResolution, Points},
    },
};

pub const MAP_W: usize = 80;
pub const MAP_H: usize = 40;

pub fn render_ascii_world_map(lat: f64, lon: f64, resolved: Option<&GeoResult>) {
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

    // Create a test backend exactly 80x40 interior + 2 for borders => 82x42
    // Block::bordered will handle the box; Canvas draws inside.
    let width = (MAP_W + 2) as u16;
    let height = (MAP_H + 2) as u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    terminal
        .draw(|frame| {
            let area: Rect = frame.area();
            let canvas = Canvas::default()
                .block(Block::new().borders(Borders::ALL).style(Style::default().fg(Color::DarkGray)))
                .x_bounds([-180.0, 180.0])
                .y_bounds([-90.0, 90.0])
                .paint(|ctx| {
                    // World map - High resolution gives detailed coastlines
                    ctx.draw(&Map {
                        color: Color::Green,
                        resolution: MapResolution::High,
                    });
                    // Marker for the queried location
                    ctx.draw(&Points {
                        coords: &[(lon, lat)],
                        color: Color::Red,
                    });
                });
            frame.render_widget(canvas, area);
        })
        .expect("failed to draw");

    // Extract buffer and print
    let buffer = terminal.backend().buffer().clone();

    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            let cell = buffer.cell((x, y)).expect("cell");
            let symbol = cell.symbol().to_string();
            let style = cell.style();

            // Color mapping based on ratatui style
            // Map is Green, Points is Red, Block borders DarkGray
            if style.fg == Some(Color::Red) {
                // Marker - always show as ★ for visibility (Points may render as braille dot)
                line.push_str(&"★".red().bold().to_string());
            } else if style.fg == Some(Color::Green) {
                // Land - keep as is but green. Ratatui uses various line chars; keep them
                // Optionally normalize to "*" if you want strict "*" for land:
                // Here we keep ratatui's detailed coastline but ensure land chars are green
                line.push_str(&symbol.green().dimmed().to_string());
            } else if symbol.chars().any(|c| "┌┐└┘─│".contains(c)) {
                line.push_str(&symbol.dimmed().to_string());
            } else {
                // Sea / background - use "." for sea (blue dimmed)
                if symbol == " " {
                    line.push_str(&".".blue().dimmed().to_string());
                } else {
                    line.push_str(&symbol.blue().dimmed().to_string());
                }
            }
        }
        // The buffer already contains the box; print line as is
        println!("{}", line);
    }

    println!("{}", "★ = location".red().dimmed());
}
