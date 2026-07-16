use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

// Custom Complex number implementation to ensure zero dependencies other than the 'image' crate
#[derive(Clone, Copy, Debug)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    // Constructor
    fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }

    // Complex addition
    fn add(self, other: Self) -> Self {
        Complex::new(self.re + other.re, self.im + other.im)
    }

    // Complex subtraction
    fn sub(self, other: Self) -> Self {
        Complex::new(self.re - other.re, self.im - other.im)
    }

    // Complex multiplication
    fn mul(self, other: Self) -> Self {
        Complex::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    // Optimized Complex Cube: z³ = (x³ - 3xy²) + i(3x²y - y³)
    fn cube(self) -> Self {
        let x2 = self.re * self.re;
        let y2 = self.im * self.im;
        Complex::new(self.re * (x2 - 3.0 * y2), self.im * (3.0 * x2 - y2))
    }

    // Complex Division: handles division by zero safely
    fn div(self, other: Self) -> Self {
        let denom = other.re * other.re + other.im * other.im;
        if denom < 1e-15 {
            Complex::new(0.0, 0.0)
        } else {
            Complex::new(
                (self.re * other.re + self.im * other.im) / denom,
                (self.im * other.re - self.re * other.im) / denom,
            )
        }
    }

    // Square of the magnitude/absolute value
    fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    // Distance between two complex coordinates
    fn dist(self, other: Self) -> f64 {
        ((self.re - other.re).powi(2) + (self.im - other.im).powi(2)).sqrt()
    }
}

// Rendering configuration constants designed to match dragon8_s_2.png
const WIDTH: u32 = 2000; // Output width (matches high-res export)
const HEIGHT: u32 = 2000; // Output height
const MAX_ITERATIONS: usize = 200; // Iteration limit for resolving fine structures
const EPSILON: f64 = 1e-5; // Convergence tolerance limit (ε)
const COLOR_SCALE: f64 = 3.2; // Exponential power gradient curve contrast
const ZOOM: f64 = 3.0; // View scale matching the original viewport
const CENTER_X: f64 = 0.0; // Complex coordinate plane X center offset
const CENTER_Y: f64 = 0.0; // Complex coordinate plane Y center offset

// The critical 'c' parameter for the triple dragon map
const C_REAL: f64 = 0.18;
const C_IMAG: f64 = 0.68;

fn main() {
    println!("======================================================");
    println!("     TRIPLE DRAGON COMPLEX FRACTAL GENERATOR (RUST)   ");
    println!("======================================================");
    println!("Formula: z_(n+1) = z_n^3 / (z_n^3 + 1) + c (from equation.png)");
    println!("Configuring canvas resolution: {} x {}", WIDTH, HEIGHT);
    println!("Target parameters: c = {} + {}i", C_REAL, C_IMAG);
    println!("Computing grayscale mapping for dragon8_s_2.png style...");

    let start_time = Instant::now();

    // Use Arc and Mutex to safely share progress monitoring across working threads
    let completed_rows = Arc::new(Mutex::new(0));

    // Allocate continuous raw buffer for flat grayscale output
    let num_pixels = (WIDTH * HEIGHT) as usize;
    let mut image_data = vec![0u8; num_pixels];

    // Determine the number of CPU threads to spawn
    let num_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    println!(
        "Spawning {} system hardware threads for render task...",
        num_threads
    );

    let mut handles = vec![];
    let chunks = image_data.chunks_mut(num_pixels / num_threads);

    // Run parallel execution
    for (thread_id, chunk) in chunks.enumerate() {
        let completed_rows = Arc::clone(&completed_rows);
        let chunk_len = chunk.len();

        // Pass chunk ownership to parallel worker
        let handle = thread::spawn(move || {
            let mut local_data = vec![0u8; chunk_len];
            let aspect = WIDTH as f64 / HEIGHT as f64;

            // Calculate pixel index scope belonging to this specific thread chunk
            let start_pixel_idx = thread_id * (num_pixels / num_threads);

            for idx in 0..chunk_len {
                let pixel_idx = start_pixel_idx + idx;
                let x = (pixel_idx % WIDTH as usize) as f64;
                let y = (pixel_idx / WIDTH as usize) as f64;

                // Normalize mapping coordinates onto the complex plane
                let uv_x = x / WIDTH as f64;
                let uv_y = 1.0 - (y / HEIGHT as f64); // Flip Y-axis to match coordinate grid

                let re = CENTER_X + (uv_x - 0.5) * ZOOM * aspect;
                let im = CENTER_Y + (uv_y - 0.5) * ZOOM;

                // Setup starting sequence value z0
                let mut z = Complex::new(re, im);
                let c = Complex::new(C_REAL, C_IMAG);

                let mut iter = MAX_ITERATIONS;
                let mut converged = false;

                // Rational sequence iteration loop
                for i in 0..MAX_ITERATIONS {
                    let z3 = z.cube();
                    let num = z3;
                    let denom = z3.add(Complex::new(1.0, 0.0));

                    // z_{n+1} = z_n³ / (z_n³ + 1) + c
                    let next_z = num.div(denom).add(c);

                    // Convergence evaluation check: |z_{n+1} - z_n| < epsilon
                    if next_z.dist(z) < EPSILON {
                        iter = i;
                        converged = true;
                        break;
                    }

                    // Escape validation step
                    if next_z.norm_sq() > 1e6 {
                        iter = i;
                        break;
                    }

                    z = next_z;
                }

                // Perfect translation of dragon8_s_2.png grayscale spectrum
                let color_val = if converged {
                    let t_linear = iter as f64 / MAX_ITERATIONS as f64;
                    // Low-iteration structures resolve to paper-white (0.96 scale), complex boundaries map to deep charcoal
                    let intensity = 0.96 - 0.96 * t_linear.powf(COLOR_SCALE);
                    (intensity.clamp(0.0, 1.0) * 255.0) as u8
                } else {
                    0 // Unconverged boundary cores are solid charcoal black
                };

                local_data[idx] = color_val;

                // Progress update block executed by thread 0
                if thread_id == 0 && idx % (WIDTH as usize * 10) == 0 {
                    let mut rows = completed_rows.lock().unwrap();
                    *rows += 10;
                    let percent = (*rows as f64 / (HEIGHT as f64 / num_threads as f64)) * 100.0;
                    print!("\rCalculating: {:.1}% complete...", percent.min(100.0));
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }
            }
            local_data
        });
        handles.push(handle);
    }

    // Collect rendered segments back into primary flat array
    let mut pointer = 0;
    for handle in handles {
        let result = handle.join().unwrap();
        let len = result.len();
        image_data[pointer..(pointer + len)].copy_from_slice(&result);
        pointer += len;
    }

    print!("\rCalculating: 100.0% complete.\n");
    let render_duration = start_time.elapsed();
    println!(
        "Fractal mathematical calculation completed in: {:?}",
        render_duration
    );

    // Save image array into structured Grayscale PNG using the 'image' crate
    let output_path = Path::new("triple_dragon_output.png");
    println!("Saving PNG to disk: {:?}", output_path);

    let save_start = Instant::now();
    image::save_buffer(
        output_path,
        &image_data,
        WIDTH,
        HEIGHT,
        image::ColorType::L8,
    )
    .expect("Failed to write buffer to output file.");

    println!("Image written successfully in: {:?}", save_start.elapsed());
    println!("Overall execution time: {:?}", start_time.elapsed());
    println!("======================================================");
    println!("Done! Check the file 'triple_dragon_output.png' in your project folder.");
}
