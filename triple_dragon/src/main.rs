use image::{ImageBuffer, Rgb};
use num_complex::Complex64;
use rayon::prelude::*;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 1200;
const MAX_ITER: u32 = 256;

// Adjust these to zoom in/out or pan around the complex plane
const X_MIN: f64 = -2.0;
const X_MAX: f64 = 2.0;
const Y_MIN: f64 = -2.0;
const Y_MAX: f64 = 2.0;

fn main() {
    println!("Generating Triple Dragon fractal...");

    // Create an image buffer
    let mut img = ImageBuffer::new(WIDTH, HEIGHT);

    // We use rayon's par_iter_mut to process rows of pixels in parallel 
    // for a massive performance boost.
    img.enumerate_rows_mut().par_bridge().for_each(|(y, row)| {
        for (x, _, pixel) in row {
            // 1. Map pixel coordinate to a complex number 'c'
            let cx = X_MIN + (x as f64 / WIDTH as f64) * (X_MAX - X_MIN);
            let cy = Y_MIN + (y as f64 / HEIGHT as f64) * (Y_MAX - Y_MIN);
            let c = Complex64::new(cx, cy);
            
            // 2. Start z at 0 (or c, depending on the fractal variation)
            let mut z = Complex64::new(0.0, 0.0);
            let mut iter = 0;

            // 3. Iterate the equation from equation.png
            while iter < MAX_ITER && z.norm_sqr() < 100.0 {
                let z3 = z.powi(3);
                // z_{n+1} = z_n^3 / (z_n^3 + 1) + c
                z = z3 / (z3 + Complex64::new(1.0, 0.0)) + c;
                iter += 1;
            }

            // 4. Color the pixel based on escape time
            *pixel = colorize(iter);
        }
    });

    // Save the result
    img.save("triple_dragon.png").unwrap();
    println!("Saved to triple_dragon.png");
}

/// A simple coloring function mapping iteration count to an RGB color.
fn colorize(iter: u32) -> Rgb<u8> {
    if iter == MAX_ITER {
        // Point is inside the set
        Rgb([0, 0, 0])
    } else {
        // Smooth out the color band based on iteration count
        let t = iter as f32 / MAX_ITER as f32;
        let r = (9.0 * (1.0 - t) * t * t * t * 255.0) as u8;
        let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u8;
        let b = (8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u8;
        
        Rgb([r, g, b])
    }
}
