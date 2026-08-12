use std::net::{IpAddr, UdpSocket};
use std::process::Command;
use std::env;

/// Retrieves the system's hostname by executing the `hostname` command.
///
/// # Returns
///
/// * `Some(String)` - The hostname if successfully retrieved and converted to UTF-8
/// * `None` - If the command fails to execute or returns a non-success status
///
/// # Panics
///
/// Panics if the `hostname` command cannot be executed at all.
fn get_hostname() -> Option<String> {
    let output = Command::new("hostname")
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(hostname)
    } else {
        None
    }
}

/// Determines the local IP address by creating a UDP socket and connecting to an external address.
///
/// This function doesn't actually send any data; it uses the socket connection to determine
/// which local IP address would be used to reach the external address (8.8.8.8).
///
/// # Returns
///
/// * `Some(IpAddr)` - The local IP address that would be used for external connections
/// * `None` - If the socket cannot be created or connected
fn get_local_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let local_addr = socket.local_addr().ok()?;
    Some(local_addr.ip())
}

/// Retrieves the public IP address by querying an external API service.
///
/// Makes an HTTP GET request to https://api.ipify.org which returns the public IP address
/// as seen from the internet.
///
/// # Returns
///
/// * `Some(IpAddr)` - The public IP address if successfully retrieved and parsed
/// * `None` - If the HTTP request fails or the response cannot be parsed as an IP address
fn get_public_ip() -> Option<IpAddr> {
    let response = reqwest::blocking::get("https://api.ipify.org").ok()?;
    let ip_str = response.text().ok()?;
    ip_str.parse().ok()
}

/// Detects the active network interface type (Ethernet or Wi-Fi) on macOS.
///
/// Uses the `networksetup -listnetworkserviceorder` command to query active network services.
///
/// # Returns
///
/// * `Some(String)` - The type of the active network interface ("Ethernet" or "Wi-Fi")
/// * `None` - If no active network interface is found or the command fails
#[cfg(target_os = "macos")]
fn get_network_type() -> Option<String> {
    let output = Command::new("networksetup")
        .arg("-listallhardwareports")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Parse the output to find active interface
    let local_ip = get_local_ip()?;
    let local_ip_str = local_ip.to_string();
    
    // Check each hardware port to see which one has our local IP
    let mut current_port = None;
    
    for line in output_str.lines() {
        if line.starts_with("Hardware Port:") {
            current_port = line.split(":").nth(1).map(|s| s.trim().to_string());
        } else if line.starts_with("Device:") {
            let current_device = line.split(":").nth(1).map(|s| s.trim().to_string());
            
            // Check if this device has our IP address
            if let Some(device) = current_device {
                let ip_output = Command::new("ipconfig")
                    .arg("getifaddr")
                    .arg(device)
                    .output()
                    .ok()?;
                    
                if ip_output.status.success() {
                    let device_ip = String::from_utf8_lossy(&ip_output.stdout).trim().to_string();
                    if device_ip == local_ip_str {
                        return current_port;
                    }
                }
            }
        }
    }
    
    None
}

#[cfg(target_os = "linux")]
fn get_network_type() -> Option<String> {
    let local_ip = get_local_ip()?;
    let local_ip_str = local_ip.to_string();

    // Use `ip -o addr` to find the interface associated with the local IP
    let output = Command::new("ip")
        .args(&["-o", "addr"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines() {
        // Line format example: "2: wlp2s0    inet 192.168.1.105/24 ..."
        if line.contains(&local_ip_str) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let interface = parts[1];
                // Check if the interface is wireless by looking for /sys/class/net/<iface>/wireless
                let wifi_path = format!("/sys/class/net/{}/wireless", interface);
                if std::path::Path::new(&wifi_path).exists() {
                   return Some("Wi-Fi".to_string());
                } else {
                   return Some("Ethernet".to_string());
                }
            }
        }
    }
    None
}

/// Retrieves the current Wi-Fi network name (SSID) on macOS.
///
/// Uses the `system_profiler` command to query the current Wi-Fi network.
///
/// # Returns
///
/// * `Some(String)` - The SSID of the current Wi-Fi network
/// * `None` - If not connected to Wi-Fi or the command fails
#[cfg(target_os = "macos")]
fn get_wifi_ssid() -> Option<String> {
    let output = Command::new("system_profiler")
        .arg("SPAirPortDataType")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Parse the output to find the current network name
    // It appears right after "Current Network Information:" line
    let mut found_current_network = false;
    for line in output_str.lines() {
        if line.contains("Current Network Information:") {
            found_current_network = true;
            continue;
        }
        
        if found_current_network {
            let trimmed = line.trim();
            // The next non-empty line after "Current Network Information:" is the SSID
            if !trimmed.is_empty() && trimmed.ends_with(':') {
                return Some(trimmed.trim_end_matches(':').to_string());
            }
        }
    }
    
    None
}

#[cfg(target_os = "linux")]
fn get_wifi_ssid() -> Option<String> {
    // Use nmcli to get the active Wi-Fi SSID
    // Command: nmcli -t -f active,ssid dev wifi
    let output = Command::new("nmcli")
        .args(&["-t", "-f", "active,ssid", "dev", "wifi"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Look for the line starting with "yes:"
    for line in output_str.lines() {
        if line.starts_with("yes:") {
            return Some(line.trim_start_matches("yes:").to_string());
        }
    }
    
    None
}

fn main() {
    println!("Operating System: {}", env::consts::OS);
    
    let hostname: Option<String> = get_hostname();
    match hostname {
        Some(name) => println!("Hostname: {}", name),
        None => println!("Could not retrieve hostname"),    
    } 

    let local_ip: Option<IpAddr> = get_local_ip();
    match local_ip {
        Some(ip) => println!("Local IP Address: {}", ip),
        None => println!("Could not retrieve local IP address"),
    }
    
    let public_ip: Option<IpAddr> = get_public_ip();
    match public_ip {
        Some(ip) => println!("Public IP Address: {}", ip),
        None => println!("Could not retrieve public IP address"),
    }
    
    let network_type = get_network_type();
    match &network_type {
        Some(net_type) => println!("Network Type: {}", net_type),
        None => println!("Could not determine network type"),
    }
    
    // If it's Wi-Fi, get the SSID
    if let Some(net_type) = network_type {
        if net_type.contains("Wi-Fi") || net_type.contains("AirPort") {
            let ssid = get_wifi_ssid();
            match ssid {
                Some(name) => println!("Wi-Fi Network: {}", name),
                None => println!("Not connected to Wi-Fi or could not retrieve SSID"),
            }
        }
    }
}
