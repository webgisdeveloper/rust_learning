use jlrs::prelude::*;

fn main() {
    // 1. Initialize Julia runtime (no `mut` needed in jlrs 0.24)
    let julia = Builder::new()
        .start_local()
        .expect("Failed to initialize Julia runtime");

    // 2. Allocate 16 GC frame slots via turbofish syntax
    julia.local_scope::<_, 16>(|mut frame| {
        let julia_script = r#"
            using Pkg
            
            if !haskey(Pkg.project().dependencies, "GLMakie")
                Pkg.add("GLMakie")
            end

            using GLMakie

            x = range(-5, 5, length=100)
            y = range(-5, 5, length=100)
            f(x, y) = sin(sqrt(x^2 + y^2))

            fig = surface(x, y, f, 
                colormap = :viridis, 
                axis = (type = Axis3, title = "3D Surface visualised via Rust + Julia")
            )

            display(fig)
            println("Rendering 3D Surface... Press Enter in terminal to exit.")
            readline()
        "#;

        // 3. Mark the dynamic string evaluation as unsafe
        unsafe {
            Value::eval_string(&mut frame, julia_script)
                .expect("Failed to execute Julia script");
        }
    });
}
