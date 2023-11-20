use crate::canvas::Canvas;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::process::exit;
/// Stores configuration information for accessing the Canvas API.
///
/// This structure holds essential data required for making authenticated requests to the Canvas API.
/// It includes the base URL of the Canvas instance and the API token used for authentication.
///
/// # Fields
///
/// - `url_canvas`: The base URL of the Canvas API endpoint.
/// - `token_canvas`: The API token used for authenticating requests to the Canvas system.
///
/// # Examples
///
/// ```
/// // Example of creating a CanvasInfo instance
/// let canvas_info = CanvasInfo {
///     url_canvas: "https://canvas.example.com".to_string(),
///     token_canvas: "your_api_token".to_string(),
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CanvasCredentials {
    pub url_canvas: String,
    pub token_canvas: String,
}

enum CanvasCredentialType {
    None,
    File(CanvasCredentials),
    System(CanvasCredentials),
}

impl CanvasCredentials {
    fn test_canvas_credentials(api_url: &str, access_token: &str) -> Result<u16, u16> {
        let client = reqwest::blocking::Client::new();
        let res = client
            .get(format!("{}/users/self", api_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .send();

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(200)
                } else {
                    Err(response.status().as_u16())
                }
            }
            Err(_) => Err(0), // Código genérico para erros de rede ou de cliente HTTP
        }
    }

    fn load_credentials() -> CanvasCredentialType {
        // Inicialmente, tenta carregar as credenciais do arquivo
        match Canvas::load_credentials_from_file() {
            Ok(credentials_ok) => {
                return CanvasCredentialType::File(credentials_ok);
            }
            Err(_) => {
                // Se não for possível carregar do arquivo, tenta carregar do sistema
                match Canvas::load_credentials_from_system() {
                    Ok(credentials_ok) => {
                        return CanvasCredentialType::System(credentials_ok);
                    }
                    Err(_) => {
                        return CanvasCredentialType::None;
                    }
                }
            }
        }
    }

    fn set_system_credentials() -> CanvasCredentialType {
        let app_name = env!("CARGO_PKG_NAME");
        loop {
            println!("Do you wish to register the credentials? You can find your API key in your Canvas Learning account settings. (y/n)");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.trim().to_uppercase() != "Y" {
                return CanvasCredentialType::None;
            }
            println!("Enter the Canvas URL:");
            input.clear();
            std::io::stdin().read_line(&mut input).unwrap();
            let url = input.trim().to_string();
            println!("Enter the Canvas token:");
            input.clear();
            std::io::stdin().read_line(&mut input).unwrap();
            let token = input.trim().to_string();

            // Update the credentials
            if let Err(e) = Entry::new(app_name, "URL_CANVAS")
                .unwrap()
                .set_password(url.as_str())
            {
                eprintln!("Error saving URL: {}", e);
            }
            if let Err(e) = Entry::new(app_name, "TOKEN_CANVAS")
                .unwrap()
                .set_password(token.as_str())
            {
                eprintln!("Error saving token: {}", e);
            }
            match Self::test_canvas_credentials(url.as_str(), token.as_str()) {
                Ok(_) => {
                    return CanvasCredentialType::System(CanvasCredentials {
                        url_canvas: url,
                        token_canvas: token,
                    });
                }
                Err(status_code) if status_code == 401 || status_code == 403 => {
                    println!("Incorrect credentials");
                }
                Err(status_code) => {
                    println!("Error accessing Canvas API - Status Code {}", status_code);
                    exit(1);
                }
            }
        }
    }

    pub fn credentials() -> CanvasCredentials {
        match Self::load_credentials() {
            CanvasCredentialType::None => match Self::set_system_credentials() {
                CanvasCredentialType::System(credentials) => {
                    return credentials;
                }
                _ => {
                    println!("Error obtaining credentials");
                    exit(1);
                }
            },
            CanvasCredentialType::File(credentials) => {
                match Self::test_canvas_credentials(
                    credentials.url_canvas.as_str(),
                    credentials.token_canvas.as_str(),
                ) {
                    Ok(_) => {
                        return CanvasCredentials {
                            url_canvas: credentials.url_canvas,
                            token_canvas: credentials.token_canvas,
                        };
                    }
                    Err(e) => {
                        println!("Error accessing Canvas API - Status Code {}", e);
                        exit(1);
                    }
                }
            }
            CanvasCredentialType::System(credentials) => {
                match Self::test_canvas_credentials(
                    credentials.url_canvas.as_str(),
                    credentials.token_canvas.as_str(),
                ) {
                    Ok(_) => {
                        return CanvasCredentials {
                            url_canvas: credentials.url_canvas,
                            token_canvas: credentials.token_canvas,
                        };
                    }
                    Err(status_code) if status_code == 401 || status_code == 403 => {
                        match Self::set_system_credentials() {
                            CanvasCredentialType::System(credentials) => {
                                return credentials;
                            }
                            _ => {
                                println!("Error obtaining credentials");
                                exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error accessing Canvas API - Status Code {}", e);
                        exit(1);
                    }
                }
            }
        }
    }
}
