use image::{ImageBuffer, Rgb};

/// A highly optimized custom complex number implementation to eliminate external math dependencies
/// and achieve extreme execution speeds during high-iteration fractal calculation loops.
#[derive(Clone, Copy, Debug)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    /// Creates a new complex number instance.
    #[inline]
    fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }

    /// Performs complex addition: (a + bi) + (c + di) = (a + c) + (b + d)i
    #[inline]
    fn add(self, other: Self) -> Self {
        Complex::new(self.re + other.re, self.im + other.im)
    }

    /// Performs complex multiplication: (a + bi) * (c + di) = (ac - bd) + (ad + bc)i
    #[inline]
    fn mul(self, other: Self) -> Self {
        Complex::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    /// Performs optimized complex cubing: z^3 = z * z^2 = (x^3 - 3xy^2) + i(3x^2y - y^3)
    #[inline]
    fn cube(self) -> Self {
        let r2 = self.re * self.re;
        let i2 = self.im * self.im;
        Complex::new(
            self.re * (r2 - 3.0 * i2),
            self.im * (3.0 * r2 - i2),
        )
    }

    /// Performs complex division: a / b = (a * conj(b)) / |b|^2
    #[inline]
    fn div(self, other: Self) -> Self {
        let denom = other.re * other.re + other.im * other.im;
        if denom < 1e-15 {
            Complex::new(0.0, 0.0) // Return zero on near-zero singularity
        } else {
            Complex::new(
                (self.re * other.re + self.im * other.im) / denom,
                (self.im * other.re - self.re * other.im) / denom,
            )
        }
    }

    /// Computes the squared distance between two complex points (much faster than square-root distance)
    #[inline]
    fn dist_sq(self, other: Self) -> f64 {
        let dr = self.re - other.re;
        let di = self.im - other.im;
        dr * dr + di * di
    }

    /// Computes the squared norm of a complex number
    #[inline]
    fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

/// Supported color themes based on the user's uploaded visual references.
#[derive(Clone, Copy, Debug)]
enum ColorTheme {
    /// Crisp, high-contrast monochrome with bright off-white background matching `dragon8_s_2.png`.
    ClassicMonochrome,
    /// Soft sky-blue backdrop with pastel pink/salmon cores and blue boundaries matching `dragon1_s.jpg`.
    PastelCoralSky,
}

/// Configuration settings for the fractal generator.
struct RenderConfig {
    width: usize,
    height: usize,
    max_iterations: usize,
    epsilon: f64,
    theme: ColorTheme,
    c: Complex,
    zoom: f64,
    center: Complex,
    color_scale: f64,
}

/// Linearly interpolates between two RGB float color slices.
#[inline]
fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Computes the final RGB pixel value depending on iteration performance and the chosen theme.
fn get_pixel_color(converged: bool, iter: usize, max_iter: usize, final_z: Complex, config: &RenderConfig) -> [u8; 3] {
    let t_linear = iter as f64 / max_iter as f64;

    match config.theme {
        ColorTheme::ClassicMonochrome => {
            if converged {
                // Background maps to light gray/white, slow convergence turns dark charcoal.
                // Recreates the light gradient curves visible in `dragon8_s_2.png`.
                let shade = t_linear.powf(config.color_scale).clamp(0.0, 1.0);
                let gray = (0.96 - 0.96 * shade) * 255.0;
                [gray as u8, gray as u8, gray as u8]
            } else {
                [0, 0, 0] // Outer unconverged bounds remain deep charcoal/black
            }
        }
        ColorTheme::PastelCoralSky => {
            if converged {
                // Color ramp anchors taken directly from the color nodes of `dragon1_s.jpg`:
                let bg_color = [0.83, 0.94, 0.99];      // Soft sky-blue
                let basin_color = [0.99, 0.91, 0.91];   // Creamy pale pink
                let coral_color = [0.98, 0.56, 0.50];   // Warm salmon/coral
                let edge_color = [0.28, 0.27, 0.51];    // Deep space indigo/violet

                let shade = t_linear.powf(config.color_scale * 0.45).clamp(0.0, 1.0);

                let mixed_color = if shade < 0.22 {
                    mix(bg_color, basin_color, shade / 0.22)
                } else if shade < 0.70 {
                    mix(basin_color, coral_color, (shade - 0.22) / 0.48)
                } else {
                    mix(coral_color, edge_color, (shade - 0.70) / 0.30)
                };

                [
                    (mixed_color[0] * 255.0) as u8,
                    (mixed_color[1] * 255.0) as u8,
                    (mixed_color[2] * 255.0) as u8,
                ]
            } else {
                // Unconverged central filaments
                [38, 28, 69] // Deep Navy/Indigo from the core of `dragon1_s.jpg`
            }
        }
    }
}

/// Computes the complex plane points and writes the resulting plot into a PNG file.
fn generate_png(config: &RenderConfig, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut imgbuf = ImageBuffer::new(config.width as u32, config.height as u32);

    let aspect = config.width as f64 / config.height as f64;
    let half_width = config.width as f64 / 2.0;
    let half_height = config.height as f64 / 2.0;

    println!("Rendering started. Resolving grid mapping...");

    for y in 0..config.height {
        for x in 0..config.width {
            // Coordinate transformation mapping the pixel grid to the Complex plane (z0)
            let uv_x = (x as f64 - half_width) / half_width;
            let uv_y = (half_height - y as f64) / half_height; // Flip y for Cartesian mapping

            let z0_re = config.center.re + uv_x * config.zoom * aspect;
            let z0_im = config.center.im + uv_y * config.zoom;
            let mut z = Complex::new(z0_re, z0_im);

            let mut converged = false;
            let mut iter_count = config.max_iterations;
            let mut final_z = z;

            // Iterate the rational map formula: z_{n+1} = z_n^3 / (z_n^3 + 1) + c
            for i in 0..config.max_iterations {
                let z3 = z.cube();
                let num = z3;
                let denom = z3.add(Complex::new(1.0, 0.0));
                
                let next_z = num.div(denom).add(config.c);

                // Convergence check: |z_{n+1} - z_n| < epsilon
                if next_z.dist_sq(z) < config.epsilon * config.epsilon {
                    iter_count = i;
                    converged = true;
                    final_z = next_z;
                    break;
                }

                // Escape guard check
                if next_z.norm_sq() > 1e6 {
                    iter_count = i;
                    final_z = next_z;
                    break;
                }

                z = next_z;
            }

            // Get colors based on theme configuration
            let rgb = get_pixel_color(converged, iter_count, config.max_iterations, final_z, config);
            imgbuf.put_pixel(x as u32, y as u32, Rgb(rgb));
        }
    }

    // Save the image directly to a PNG file
    imgbuf.save(filename)?;
    println!("Render successfully completed! Output file written: {}", filename);
    Ok(())
}

fn main() {
    // 1. SETUP CLASSSIC MONOCHROME CONFIGURATION (dragon8_s_2.png)
    let monochrome_config = RenderConfig {
        width: 2000,
        height: 2000,
        max_iterations: 200,
        epsilon: 1e-5,
        theme: ColorTheme::ClassicMonochrome,
        c: Complex::new(0.18, 0.68), // Standard coordinates
        zoom: 1.25,
        center: Complex::new(0.0, 0.0),
        color_scale: 3.2, // Optimized for bright light-gray backdrop
    };

    // 2. SETUP PASTEL CORAL SKY CONFIGURATION (dragon1_s.jpg)
    let coral_sky_config = RenderConfig {
        width: 2000,
        height: 2000,
        max_iterations: 200,
        epsilon: 1e-5,
        theme: ColorTheme::PastelCoralSky,
        c: Complex::new(0.18, 0.68),
        zoom: 1.25,
        center: Complex::new(0.0, 0.0),
        color_scale: 2.8, // Optimized for vibrant pastel gradient thresholds
    };

    println!("===========================================================");
    println!("     TRIPLE DRAGON FRACTAL GENERATOR - RUST PNG ENGINE     ");
    println!("===========================================================");
    println!("Formula verbatim from equation.png: z_next = z^3 / (z^3 + 1) + c");
    println!("Output format: High-Resolution Native PNG");
    println!("-----------------------------------------------------------");

    // Render Monochrome (dragon8_s_2.png theme)
    println!("Executing Render [1/2]: Classic Monochrome...");
    if let Err(e) = generate_png(&monochrome_config, "triple_dragon_monochrome.png") {
        eprintln!("Error writing monochrome plot: {:?}", e);
    }

    // Render Pastel Coral Sky (dragon1_s.jpg theme)
    println!("Executing Render [2/2]: Pastel Coral Sky...");
    if let Err(e) = generate_png(&coral_sky_config, "triple_dragon_pastel.png") {
        eprintln!("Error writing pastel plot: {:?}", e);
    }

    println!("-----------------------------------------------------------");
    println!("Done! All PNG files have been written directly to your directory.");
    println!("===========================================================");
}
