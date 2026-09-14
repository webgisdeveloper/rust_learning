use jlrs::prelude::*;

/// Target mathematical function f(x, y) = sin(sqrt(x^2 + y^2)) computed in Rust
fn f(x: f64, y: f64) -> f64 {
    (x.powi(2) + y.powi(2)).sqrt().sin()
}

fn main() {
    // 1. Compute 3D point data (x, y, z = f(x, y)) in Rust
    let n_per_dim = 60;
    let total_points = n_per_dim * n_per_dim;

    let min_val = -5.0;
    let max_val = 5.0;

    let mut x_vals = Vec::with_capacity(total_points);
    let mut y_vals = Vec::with_capacity(total_points);
    let mut z_vals = Vec::with_capacity(total_points);

    for i in 0..n_per_dim {
        let x = min_val + (max_val - min_val) * (i as f64) / ((n_per_dim - 1) as f64);
        for j in 0..n_per_dim {
            let y = min_val + (max_val - min_val) * (j as f64) / ((n_per_dim - 1) as f64);
            let z = f(x, y);

            x_vals.push(x);
            y_vals.push(y);
            z_vals.push(z);
        }
    }

    // 2. Initialize Julia runtime
    let julia = Builder::new()
        .start_local()
        .expect("Failed to initialize Julia runtime");

    // 3. Allocate GC frame slots
    julia.local_scope::<_, 16>(|mut frame| {
        // Convert Rust data arrays into Julia vectors
        let x_jl = TypedVector::<f64>::from_vec(&mut frame, x_vals, total_points)
            .expect("Failed to allocate Julia array x")
            .expect("Failed to construct Julia array x");
        let y_jl = TypedVector::<f64>::from_vec(&mut frame, y_vals, total_points)
            .expect("Failed to allocate Julia array y")
            .expect("Failed to construct Julia array y");
        let z_jl = TypedVector::<f64>::from_vec(&mut frame, z_vals, total_points)
            .expect("Failed to allocate Julia array z")
            .expect("Failed to construct Julia array z");

        // Define Julia package imports and 3D scatter plotting function
        let julia_script = r#"
            using Pkg
            
            if !haskey(Pkg.project().dependencies, "GLMakie")
                Pkg.add("GLMakie")
            end

            using GLMakie

            function plot_3d_points(x, y, z)
                fig = scatter(x, y, z, 
                    markersize = 4.0,
                    colormap = :viridis, 
                    color = z,
                    axis = (type = Axis3, title = "3D Points visualised via Rust + Julia")
                )

                display(fig)
                println("Rendering 3D Points... Press Enter in terminal to exit.")
                readline()
            end
        "#;

        unsafe {
            Value::eval_string(&mut frame, julia_script)
                .expect("Failed to execute Julia setup script");
        }

        // Call the Julia plotting function with Rust-computed data
        let main_module = Module::main(&frame);
        let plot_func = main_module
            .global(&mut frame, "plot_3d_points")
            .expect("Failed to find plot_3d_points function in Julia Main module");

        unsafe {
            plot_func
                .call(
                    &mut frame,
                    [x_jl.as_value(), y_jl.as_value(), z_jl.as_value()],
                )
                .expect("Failed to call plot_3d_points");
        }
    });
}
