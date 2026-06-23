// Fixing the Borrow Checker

struct AppConfig {
    server_name: String,
    port: u32,
}

fn main() {
    let mut config = AppConfig {
        server_name: String::from("Actix-Production-Worker"),
        port: 8080,
    };

    print_config(&config);
    update_port(&mut config, 9000);

    println!("Server running on port: {}",config.port);
}

fn print_config (cfg: &AppConfig) {
    println!("Server {} running on port {}", cfg.server_name, cfg.port);
}

fn update_port (cfg: &mut AppConfig,new_port: u32) {
    cfg.port = new_port;
    println!("Update port to {}", new_port);
}
