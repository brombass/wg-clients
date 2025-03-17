use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use rand::Rng;
use base64::{Engine as _, engine};
use qrcode::QrCode;
use image::Luma;

#[derive(Serialize, Deserialize, Debug)]
struct ServerConfig {
    host: String,
    port: String,
    dns: String,
    subnet: String,
    public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClientConfig {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    private_key: Option<String>,
    address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    server: ServerConfig,
    client: Vec<ClientConfig>,
}

/// Generates a WireGuard private key (32 random bytes encoded in Base64)
fn generate_private_key() -> String {
    let mut rng = rand::rng();
    let mut key = [0u8; 32];
    rng.fill(&mut key);
    engine::general_purpose::STANDARD.encode(key) // Use STANDARD.encode
}

fn generate_client_config(client: &ClientConfig, server: &ServerConfig) -> String {
    let address = server.subnet.replace("{address}", &client.address);
    let private_key = client.private_key.as_deref().unwrap_or_else(|| {
        panic!("Private key not provided for client {}", client.name);
    });

    format!(
        "[Interface]\n\
        PrivateKey = {}\n\
        Address = {}\n\
        DNS = {}\n\n\
        [Peer]\n\
        PublicKey = {}\n\
        Endpoint = {}:{}\n\
        AllowedIPs = 0.0.0.0/0\n\
        PersistentKeepalive = 25\n",
        private_key,
        address,
        server.dns,
        server.public_key,
        server.host,
        server.port
    )
}

fn generate_qr_code_png(config_content: &str, output_path: &str) {
    let code = QrCode::new(config_content.as_bytes()).unwrap();
    let image = code.render::<Luma<u8>>().build();
    image.save(output_path).unwrap();
}

fn main() {
    // Read command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check if the config file path is provided
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_config.json>", args[0]);
        return;
    }

    let config_path = &args[1];

    // Read the JSON file
    let config_data = match fs::read_to_string(config_path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to read config file: {}", err);
            return;
        }
    };

    // Parse the JSON data into the Config struct
    let mut config: Config = match serde_json::from_str(&config_data) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to parse config file: {}", err);
            return;
        }
    };

    // Directory to save all output files
    let output_dir = "wg-clients";

    // Create the output directory if it doesn't exist
    if !Path::new(output_dir).exists() {
        if let Err(err) = fs::create_dir(output_dir) {
            eprintln!("Failed to create output directory: {}", err);
            return;
        }
    }

    // Generate private keys for clients if not provided
    for client in &mut config.client {
        if client.private_key.is_none() {
            client.private_key = Some(generate_private_key());
            println!("Generated private key for client {}", client.name);
        }
    }

    // Generate a configuration file and QR code for each client
    for client in &config.client {
        let config_content = generate_client_config(client, &config.server);
        let config_filename = format!("{}/{}.conf", output_dir, client.name);

        // Save the configuration file
        if let Err(err) = fs::write(&config_filename, &config_content) {
            eprintln!("Failed to write configuration for {}: {}", client.name, err);
        } else {
            println!("Generated configuration for {} at {}", client.name, config_filename);
        }

        // Generate and save the QR code as a PNG image
        let qr_code_filename = format!("{}/{}_qr.png", output_dir, client.name);
        generate_qr_code_png(&config_content, &qr_code_filename);
        println!("Generated QR code for {} at {}", client.name, qr_code_filename);
    }

    // Save the updated JSON with generated private keys
    let updated_config_path = format!("{}/updated_config.json", output_dir);
    if let Err(err) = fs::write(&updated_config_path, serde_json::to_string_pretty(&config).unwrap()) {
        eprintln!("Failed to save updated config file: {}", err);
    } else {
        println!("Saved updated config file at {}", updated_config_path);
    }
}
