//! New management certificate command implementation

use crate::auth::{hash_password, read_password};
use anyhow::Result;
use serde_json::json;

pub async fn run(server_url: &str, sans: &str) -> Result<()> {
    println!("Rotating management certificate...");
    println!();

    // Get password from user
    let password = read_password("Enter ACP password: ")?;
    let password_hash = hash_password(&password);

    // Parse SANs from comma-separated string
    let sans_vec: Vec<String> = sans
        .split(',')
        .map(|san| san.trim().to_string())
        .collect();

    // Create API client
    let client = crate::create_api_client(server_url)?;

    // Call rotate endpoint
    let body = json!({
        "sans": sans_vec,
    });

    let response: crate::client::RotateManagementCertResponse = client
        .post_auth("/v1/management-cert", &password_hash, body)
        .await?;

    println!();
    if response.rotated {
        println!("Management certificate rotated successfully!");
        println!("New SANs: {}", response.sans.join(", "));
        println!();
        println!("Note: New connections will use the new certificate.");
        println!("Existing connections will continue to work until they reconnect.");
    } else {
        println!("Failed to rotate management certificate.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_sans() {
        // Test parsing comma-separated SANs
        let input = "DNS:localhost,IP:127.0.0.1";
        let result: Vec<String> = input.split(',')
            .map(|san| san.trim().to_string())
            .collect();

        assert_eq!(result, vec!["DNS:localhost", "IP:127.0.0.1"]);
    }

    #[test]
    fn test_parse_sans_with_spaces() {
        // Test parsing with extra whitespace
        let input = " DNS:localhost , IP:127.0.0.1 , DNS:example.com ";
        let result: Vec<String> = input.split(',')
            .map(|san| san.trim().to_string())
            .collect();

        assert_eq!(result, vec!["DNS:localhost", "IP:127.0.0.1", "DNS:example.com"]);
    }

    #[test]
    fn test_parse_sans_single() {
        // Test single SAN
        let input = "DNS:localhost";
        let result: Vec<String> = input.split(',')
            .map(|san| san.trim().to_string())
            .collect();

        assert_eq!(result, vec!["DNS:localhost"]);
    }

    #[test]
    fn test_parse_sans_multiple() {
        // Test multiple SANs
        let input = "DNS:localhost,DNS:example.com,IP:127.0.0.1,IP:192.168.1.1";
        let result: Vec<String> = input.split(',')
            .map(|san| san.trim().to_string())
            .collect();

        assert_eq!(result, vec!["DNS:localhost", "DNS:example.com", "IP:127.0.0.1", "IP:192.168.1.1"]);
    }
}
