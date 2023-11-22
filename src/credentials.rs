// Import necessary crates and modules
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::process::exit;
use std::fs::File;
use std::io::BufReader;

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
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CanvasCredentials {
    pub url_canvas: String,
    pub token_canvas: String,
}

// Enum to represent the source of Canvas credentials.
enum CanvasCredentialType {
    None, // No credentials available
    File(CanvasCredentials), // Credentials loaded from a file
    System(CanvasCredentials), // Credentials loaded from system's keyring
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

    /// Loads Canvas credentials from a configuration file.
    ///
    /// Attempts to read a `config.json` file from the user's `Downloads` directory and deserialize the contents
    /// into `CanvasCredentials`.
    ///
    /// Returns:
    /// - `Ok(CanvasCredentials)`: Loaded credentials if successful.
    /// - `Err(String)`: Error message if the file can't be read or deserialized.
    #[allow(unreachable_code)]
    pub fn load_credentials_from_file() -> Result<CanvasCredentials, String> {
        // Check if the feature for using file credentials is enabled
        #[cfg(not(feature = "use_file_credentials"))]{
            return Err("Feature not enabled".to_string());
        }
        // Attempt to locate the config file in the user's home directory
        if let Some(mut home_config_buffer) = dirs::home_dir() {
            home_config_buffer.push("Downloads");
            home_config_buffer.push("config.json");

            // If the file path is valid
            if let Some(config_path) = home_config_buffer.to_str() {
                // Attempt to open the file
                if let Ok(file) = File::open(config_path) {
                    println!("Configuration file found: {}", config_path);
                    let reader = BufReader::new(file);
                    // Deserialize the JSON content into `CanvasCredentials`
                    let config: Result<CanvasCredentials, serde_json::Error> =
                        serde_json::from_reader(reader);
                    match config {
                        Ok(config) => Ok(config),
                        Err(_) => panic!("Error reading config.json"),
                    }
                } else {
                    Err("Error opening configuration file".to_string())
                }
            } else {
                panic!("Error converting path to string")
            }
        } else {
            panic!("Error obtaining home directory")
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

    /// Loads the Canvas credentials, attempting first from a file, then from the system's keyring.
    ///
    /// This function tries to load the Canvas credentials first from a configuration file and,
    /// if that fails, from the system's keyring.
    ///
    /// Returns:
    /// - `CanvasCredentialType`: Enum variant representing the source of loaded credentials.
    fn load_credentials() -> CanvasCredentialType {
        // Try loading from file
        match Self::load_credentials_from_file() {
            Ok(credentials) => CanvasCredentialType::File(credentials),
            Err(_) => {
                // If loading from file fails, try loading from system
                match Self::load_credentials_from_system() {
                    Ok(credentials) => CanvasCredentialType::System(credentials),
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
            if let Err(e) = Entry::new(app_name, "URL_CANVAS").unwrap().set_password(&url) {
                eprintln!("Error saving URL: {}", e);
                continue;
            }
            if let Err(e) = Entry::new(app_name, "TOKEN_CANVAS").unwrap().set_password(&token) {
                eprintln!("Error saving token: {}", e);
                continue;
            }

            // Validate the credentials with Canvas API
            match Self::test_canvas_credentials(&url, &token) {
                Ok(_) => {
                    return CanvasCredentialType::System(CanvasCredentials {
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
                    CanvasCredentialType::System(credentials) => credentials,
                    _ => {
                        println!("Error obtaining credentials");
                        exit(1);
                    }
                }
            },
            CanvasCredentialType::File(credentials) | CanvasCredentialType::System(credentials) => {
                // If credentials are found, validate them
                match Self::test_canvas_credentials(&credentials.url_canvas, &credentials.token_canvas) {
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
