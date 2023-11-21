use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::process::exit;
use std::fs::File;
use std::io::BufReader;

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

    /// Loads Canvas credentials from a configuration file.
    ///
    /// This function attempts to read Canvas API credentials from a predefined configuration file.
    /// It looks for a file named `config.json` in the user's `Downloads` directory and tries to deserialize
    /// the contents into a `CanvasInfo` structure. The `CanvasInfo` contains the base URL and API token required
    /// for authenticating Canvas API requests.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(CanvasInfo)` containing the loaded credentials on successful file reading and deserialization.
    /// - `Err(String)` with an error message if the file cannot be read, the path is invalid, or the contents
    ///   cannot be deserialized into `CanvasInfo`.
    ///
    /// # Examples
    ///
    /// ```
    /// match load_credentials_from_file() {
    ///     Ok(canvas_info) => println!("CanvasInfo loaded: {:?}", canvas_info),
    ///     Err(e) => eprintln!("Error loading credentials: {}", e),
    /// }
    /// ```
    #[allow(unreachable_code)]
    pub fn load_credentials_from_file() -> Result<CanvasCredentials, String> {
        #[cfg(not(feature = "use_file_credentials"))]{
            return Err("Feature not enabled".to_string());
        }
        if let Some(mut home_config_buffer) = dirs::home_dir() {
            home_config_buffer.push("Downloads");
            home_config_buffer.push("config.json");
            if let Some(config_path) = home_config_buffer.to_str() {
                if let Ok(file) = File::open(config_path) {
                    println!("Configuration file found: {}", config_path);
                    let reader = BufReader::new(file);
                    let config: Result<CanvasCredentials, serde_json::Error> =
                        serde_json::from_reader(reader);
                    if let Ok(config) = config {
                        return Ok(config);
                    } else {
                        panic!("Error reading config.json");
                    }
                } else {
                    return Err("Error opening configuration file".to_string());
                }
            } else {
                panic!("Error converting path to string");
            }
        }
        panic!("Error obtaining home directory");
    }

    /// Loads Canvas credentials from the system's keyring.
    ///
    /// This function retrieves the Canvas API credentials (URL and token) stored in the system's keyring.
    /// It uses the `keyring` crate to access the secure storage provided by the operating system. The credentials
    /// are expected to be stored under the application's name, fetched from the `CARGO_PKG_NAME` environment variable.
    /// This approach enhances security by avoiding plain text storage of sensitive information.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(CanvasInfo)` containing the credentials if successfully retrieved from the system's keyring.
    /// - `Err(String)` with an error message if there are issues accessing the keyring or retrieving the credentials.
    ///
    /// # Examples
    ///
    /// ```
    /// match load_credentials_from_system() {
    ///     Ok(canvas_info) => println!("CanvasInfo loaded from system: {:?}", canvas_info),
    ///     Err(e) => eprintln!("Error loading credentials from system: {}", e),
    /// }
    /// ```
    pub fn load_credentials_from_system() -> Result<CanvasCredentials, String> {
        let app_name = env!("CARGO_PKG_NAME");
        // Initially retrieves the URL
        match Entry::new(app_name, "URL_CANVAS") {
            Ok(entry) => {
                match entry.get_password() {
                    Ok(url) => {
                        // Retrieves the TOKEN
                        match Entry::new(app_name, "TOKEN_CANVAS") {
                            Ok(entry) => match entry.get_password() {
                                Ok(token) => {
                                    return Ok(CanvasCredentials {
                                        url_canvas: url,
                                        token_canvas: token,
                                    });
                                }
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

    fn load_credentials() -> CanvasCredentialType {
        // Inicialmente, tenta carregar as credenciais do arquivo
        match Self::load_credentials_from_file() {
            Ok(credentials_ok) => {
                return CanvasCredentialType::File(credentials_ok);
            }
            Err(_) => {
                // Se não for possível carregar do arquivo, tenta carregar do sistema
                match Self::load_credentials_from_system() {
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
