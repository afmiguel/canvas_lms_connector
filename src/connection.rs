// Import of custom module `CanvasCredentials` from the crate's root.
// This module likely contains structures or functions related to authentication or configuration
// for interacting with the Canvas API or a similar service.
use crate::CanvasCredentials;

// Import of the `lazy_static` macro.
// This macro is used for defining static variables that are initialized lazily.
// Lazy statics are beneficial in Rust as the language does not support static variables
// with complex initializations at runtime by default.
use lazy_static::lazy_static;

// Import of `Semaphore` from the `std_semaphore` crate.
// A semaphore is a synchronization primitive that can be used to control access
// to a common resource by multiple threads.
use std_semaphore::Semaphore;

/// The maximum number of simultaneous HTTP requests allowed.
///
/// This constant is crucial for controlling the load on the server and preventing
/// overloading the network with too many concurrent requests. It's used in conjunction
/// with a semaphore to limit the number of active HTTP requests at any given time.
/// Adjusting this value should be based on server capabilities and application requirements.
const SIMULTANEOUS_REQUESTS_LIMIT: isize = 20;

// The number of attempts to make for an HTTP request before giving up.
//
// This constant defines the maximum number of attempts to make for an HTTP request
// before considering it a failure. It's used in the retry logic to handle transient
// network issues or server-side errors. The value of 10 is a common choice, but it
// can be adjusted based on the application's requirements and the nature of the requests.
pub const SYNC_ATTEMPT: u32 = 10;

/// Enumeration representing the types of HTTP request methods.
///
/// This enum is used throughout the application to specify the HTTP method for requests.
/// It supports different methods like GET and PUT, with the ability to add more variants
/// as needed. The `Put` variant here demonstrates how additional data (like a JSON body)
/// can be associated with specific request types.
///
/// Note: Using an enum for HTTP methods allows for type-safe and clear representation of
/// different request types, improving code readability and maintainability.
#[derive(Clone)]
pub enum HttpMethod {
    Get,
    Put(serde_json::Value),
    Post(serde_json::Value),
    Delete,
}

// Type alias for HTTP request results.
// This alias simplifies the type signatures throughout the code and encapsulates
// the result of an HTTP request, which is either a successful `reqwest::blocking::Response`
// or an error represented by a `u16` status code.
pub type HttpRequestResult = Result<reqwest::blocking::Response, u16>;

// Global semaphore for managing simultaneous HTTP requests.
//
// This lazy_static declaration ensures that the semaphore is initialized once
// and remains in memory for the duration of the program. The semaphore's count is
// set to the `SIMULTANEOUS_REQUESTS_LIMIT`, thereby enforcing the limit on the number
// of concurrent HTTP requests.
//
// Note: The use of a global semaphore is a common pattern for controlling access
// to a limited resource (like network bandwidth) in a multi-threaded environment.
lazy_static! {
    static ref SEMAPHORE: Semaphore = Semaphore::new(SIMULTANEOUS_REQUESTS_LIMIT);
}

/// Sends an HTTP request with a single attempt.
///
/// This function constructs and sends an HTTP request based on the provided parameters.
/// It uses the `reqwest` blocking client, suitable for synchronous contexts. The request
/// method and other parameters are encapsulated in the `HttpMethod` enum and other arguments.
///
/// Error handling is basic, with network or client errors resulting in a generic error code (0).
/// This function is designed to be called within a retry loop implemented in `send_http_request`.

fn send_http_request_single_attempt(
    method: HttpMethod,
    url: &str,
    canvas_info: &CanvasCredentials,
    params: Vec<(String, String)>,
) -> HttpRequestResult {
    // Construir a requisição com base no método HTTP
    let request_builder = match &method {
        HttpMethod::Get => canvas_info
            .client
            .get(url)
            .bearer_auth(&canvas_info.token_canvas)
            .query(&params),
        HttpMethod::Put(body) => canvas_info
            .client
            .put(url)
            .bearer_auth(&canvas_info.token_canvas)
            .json(body),
        HttpMethod::Post(body) => canvas_info
            .client
            .post(url)
            .bearer_auth(&canvas_info.token_canvas)
            .json(body),
        HttpMethod::Delete => canvas_info
            .client
            .delete(url)
            .bearer_auth(&canvas_info.token_canvas)
            .query(&params), // DELETE também pode usar parâmetros de consulta
    };

    // Enviar a requisição e verificar a resposta
    let response = request_builder.send();

    match response {
        Ok(response) if response.status().is_success() => Ok(response),
        Ok(response) => Err(response.status().as_u16()),
        Err(_) => Err(0), // Código de erro genérico para falhas na requisição
    }
}

/// Sends an HTTP request with retry logic.
///
/// This function attempts to send an HTTP request multiple times (up to `max_attempts`)
/// in case of failure. It's particularly useful for handling transient network issues
/// or temporary server-side errors. A delay is introduced between retries for 403 errors,
/// which often represent rate limiting or similar temporary restrictions.
///
/// Note: This retry mechanism is a common pattern in network programming, especially
/// when interacting with external APIs that may have rate limits or occasional downtime.
use std::io;

pub fn send_http_request(
    method: HttpMethod,
    url: &str,
    canvas_info: &CanvasCredentials,
    params: Vec<(String, String)>,
) -> Result<reqwest::blocking::Response, Box<dyn std::error::Error>> {
    let mut attempts = 0;
    let max_attempts = 5;

    // Retry loop.
    while attempts < max_attempts {
        match send_http_request_single_attempt(method.clone(), url, canvas_info, params.clone()) {
            Ok(response) => return Ok(response),
            Err(status) if status == 403 && attempts < max_attempts - 1 => {
                // Retry for 403 status codes.
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(1000)); // Wait before retrying.
            }
            Err(status) => {
                // Convert the status code to a proper error type.
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    format!("HTTP request failed with status code: {}", status),
                )));
            }
        }
    }

    // Return an error after all attempts fail.
    Err(Box::new(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "All retry attempts failed with status 403",
    )))
}
