// Import necessary crates and modules
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::process::exit;

/// Structure to hold Canvas API credentials.
///
/// This struct is used to store the base URL and API token required for accessing the Canvas API.
///
/// Fields:
/// - `url_canvas`: Base URL for the Canvas API.
/// - `token_canvas`: API token for authentication.
///
/// Example usage:
/// ```
/// let canvas_credentials = CanvasCredentials {
///     url_canvas: "https://canvas.example.com".to_string(),
///     token_canvas: "your_api_token".to_string(),
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct CanvasCredentials {
    pub url_canvas: String,
    pub token_canvas: String,
}

// Enum to represent the source of Canvas credentials.
enum CanvasCredentialType {
    None,                      // No credentials available
    EnvVariables(CanvasCredentials),   // Credentials loaded from a file
    SystemKeyring(CanvasCredentials), // Credentials loaded from system's keyring
}

impl CanvasCredentials {
    /// Tests the validity of Canvas API credentials.
    ///
    /// Performs a GET request to the Canvas API to verify if the provided credentials are valid.
    ///
    /// Arguments:
    /// - `api_url`: The Canvas API URL.
    /// - `access_token`: The API token for authentication.
    ///
    /// Returns:
    /// - `Ok(200)`: If credentials are valid.
    /// - `Err(u16)`: The HTTP status code if credentials are invalid or any network error (0 for generic errors).
    fn test_canvas_credentials(api_url: &str, access_token: &str) -> Result<u16, u16> {
        // Create a blocking HTTP client
        let client = reqwest::blocking::Client::new();
        // Attempt to perform a GET request to the Canvas API
        let res = client
            .get(format!("{}/users/self", api_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .send();

        // Handle the response
        match res {
            Ok(response) => {
                // Check if the response status is successful
                if response.status().is_success() {
                    Ok(200)
                } else {
                    Err(response.status().as_u16())
                }
            }
            // Handle any network or client errors
            Err(_) => Err(0),
        }
    }

    // Função que carrega as credenciais do Canvas das variáveis do ambiente. Retorna um Result<CanvasCredentials, String>
    // com o resultado da operação. Se as credenciais não forem encontradas, retorna um erro.
    pub fn load_credentials_from_env() -> Result<CanvasCredentials, String> {
        // Check if the feature for using file credentials is enabled
        #[cfg(not(feature = "use_env_credentials"))]
        {
            return Err("Feature not enabled".to_string());
        }

        #[cfg(feature = "use_env_credentials")]
        {
            // Environment variables are used to store the credentials
            match std::env::var("CANVAS_URL") {
                Ok(url) => match std::env::var("CANVAS_TOKEN") {
                    Ok(token) => {
                        println!("Credentials loaded from environment! -> {}", url);
                        Ok(CanvasCredentials {
                            url_canvas: url,
                            token_canvas: token,
                        })
                    },
                    Err(_) => Err("Error retrieving token from environment".to_string()),
                },
                Err(_) => Err("Error retrieving URL from environment".to_string()),
            }
        }
    }

    /// Loads Canvas credentials from the system's keyring.
    ///
    /// Retrieves Canvas API credentials (URL and token) from the system's keyring.
    ///
    /// Returns:
    /// - `Ok(CanvasCredentials)`: Credentials if successfully retrieved.
    /// - `Err(String)`: Error message if issues occur accessing the keyring or retrieving credentials.
    pub fn load_credentials_from_system() -> Result<CanvasCredentials, String> {
        let app_name = env!("CARGO_PKG_NAME");
        // Retrieve the URL from the keyring
        match Entry::new(app_name, "URL_CANVAS") {
            Ok(entry) => {
                match entry.get_password() {
                    Ok(url) => {
                        // Retrieve the token from the keyring
                        match Entry::new(app_name, "TOKEN_CANVAS") {
                            Ok(entry) => match entry.get_password() {
                                Ok(token) => Ok(CanvasCredentials {
                                    url_canvas: url,
                                    token_canvas: token,
                                }),
                                Err(_) => Err("Error retrieving token from system".to_string()),
                            },
                            Err(_) => Err("Error retrieving token from system".to_string()),
                        }
                    }
                    Err(_) => Err("Error retrieving URL from system".to_string()),
                }
            }
            Err(_) => Err("Error retrieving URL from system".to_string()),
        }
    }

    /// Loads the Canvas credentials, attempting first from environment variables, then from the system's keyring.
    ///
    /// This function tries to load the Canvas credentials first from environment variables and,
    /// if that fails, from the system's keyring.
    ///
    /// Returns:
    /// - `CanvasCredentialType`: Enum variant representing the source of loaded credentials.
    fn load_credentials() -> CanvasCredentialType {
        // Try loading from environment variables
        match Self::load_credentials_from_env() {
            Ok(credentials) => CanvasCredentialType::EnvVariables(credentials),
            Err(_) => {
                // If loading from file fails, try loading from system
                match Self::load_credentials_from_system() {
                    Ok(credentials) => CanvasCredentialType::SystemKeyring(credentials),
                    Err(_) => CanvasCredentialType::None, // Return None if both methods fail
                }
            }
        }
    }

    /// Interactively sets and stores Canvas credentials in the system's keyring.
    ///
    /// This function prompts the user to manually enter the Canvas API credentials and stores
    /// them in the system's keyring. It also validates the credentials against the Canvas API.
    ///
    /// Returns:
    /// - `CanvasCredentialType`: Enum variant indicating the stored credential type.
    fn set_system_credentials() -> CanvasCredentialType {
        let app_name = env!("CARGO_PKG_NAME");
        loop {
            // Prompt user to enter credentials
            println!("Do you wish to register the credentials? (y/n)");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.trim().to_uppercase() != "Y" {
                return CanvasCredentialType::None; // Exit if user chooses not to enter credentials
            }
            // Get URL and token from user input
            println!("Enter the Canvas URL:");
            input.clear();
            std::io::stdin().read_line(&mut input).unwrap();
            let url = input.trim().to_string();
            println!("Enter the Canvas token:");
            input.clear();
            std::io::stdin().read_line(&mut input).unwrap();
            let token = input.trim().to_string();

            // Save entered credentials to the system's keyring
            if let Err(e) = Entry::new(app_name, "URL_CANVAS")
                .unwrap()
                .set_password(&url)
            {
                eprintln!("Error saving URL: {}", e);
                continue;
            }
            if let Err(e) = Entry::new(app_name, "TOKEN_CANVAS")
                .unwrap()
                .set_password(&token)
            {
                eprintln!("Error saving token: {}", e);
                continue;
            }

            // Validate the credentials with Canvas API
            match Self::test_canvas_credentials(&url, &token) {
                Ok(_) => {
                    return CanvasCredentialType::SystemKeyring(CanvasCredentials {
                        url_canvas: url,
                        token_canvas: token,
                    });
                }
                Err(status_code) if status_code == 401 || status_code == 403 => {
                    println!("Incorrect credentials");
                    continue;
                }
                Err(status_code) => {
                    println!("Error accessing Canvas API - Status Code {}", status_code);
                    exit(1);
                }
            }
        }
    }

    /// Retrieves Canvas credentials, using either stored credentials or prompting the user to input them.
    ///
    /// This method is the primary interface for obtaining Canvas API credentials. It first attempts to load
    /// existing credentials. If no credentials are found or they are invalid, it prompts the user to input new ones.
    ///
    /// Returns:
    /// - `CanvasCredentials`: The CanvasCredentials struct with the URL and token.
    pub fn credentials() -> CanvasCredentials {
        // Try loading existing credentials
        match Self::load_credentials() {
            CanvasCredentialType::None => {
                // If no credentials are found, prompt user to input them
                match Self::set_system_credentials() {
                    CanvasCredentialType::SystemKeyring(credentials) => credentials,
                    _ => {
                        println!("Error obtaining credentials");
                        exit(1);
                    }
                }
            }
            CanvasCredentialType::EnvVariables(credentials) | CanvasCredentialType::SystemKeyring(credentials) => {
                // If credentials are found, validate them
                match Self::test_canvas_credentials(
                    &credentials.url_canvas,
                    &credentials.token_canvas,
                ) {
                    Ok(_) => credentials,
                    Err(e) => {
                        println!("Error accessing Canvas API - Status Code {}", e);
                        exit(1);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_credentials_initialization() {
        let url = String::from("https://example.com");
        let token = String::from("secret-token");

        let credentials = CanvasCredentials {
            url_canvas: url,
            token_canvas: token,
        };

        assert_eq!(credentials.url_canvas, "https://example.com");
        assert_eq!(credentials.token_canvas, "secret-token");
    }

    #[test]
    #[cfg(feature = "use_env_credentials")]
    fn test_load_credentials_from_env() {
        use std::collections::HashMap;
        use std::env;

        let mut map: HashMap<String, String> = HashMap::new();
        fn set_new_key(map: &mut HashMap<String, String>, key: &str, value: &str) {
            if let Ok(value) = env::var(key) {
                map.insert(key.to_string(), value);
            }
            env::set_var(key, value);
        }

        fn restore_key(map: &HashMap<String, String>, key: &str) {
            if let Some(value) = map.get(key) {
                env::set_var(key, value);
            } else {
                env::remove_var(key);
            }
        }

        let cavas_url_key = "CANVAS_URL";
        let canvas_token_key = "CANVAS_TOKEN";

        set_new_key(&mut map, cavas_url_key, "https://example.com");
        set_new_key(&mut map, canvas_token_key, "secret-token");

        // Test both variables set
        let both_credentials = CanvasCredentials::load_credentials_from_env();

        // Test only URL set
        env::remove_var(canvas_token_key);
        let only_url = CanvasCredentials::load_credentials_from_env();

        // Teste only token set
        env::remove_var(cavas_url_key);
        env::set_var(canvas_token_key, "secret-token");
        let only_token = CanvasCredentials::load_credentials_from_env();

        // Test no variables set
        env::remove_var(cavas_url_key);
        let no_credentials = CanvasCredentials::load_credentials_from_env();

        restore_key(&map, canvas_token_key);
        restore_key(&map, cavas_url_key);

        assert!(both_credentials.is_ok());
        assert!(only_url.is_err());
        assert!(only_token.is_err());
        assert!(no_credentials.is_err());
    }
}
