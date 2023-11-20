use crate::CanvasCredentials;
use lazy_static::lazy_static;
use std::thread;
use std::time::Duration;
use std_semaphore::Semaphore;

/// The maximum number of simultaneous HTTP requests allowed.
///
/// This constant defines a limit on the number of HTTP requests that can be
/// sent concurrently. It is used to control the load on the server and prevent
/// overwhelming the network with too many simultaneous requests.
///
/// The limit is set considering the typical server load and performance
/// expectations. Adjust the value based on the specific requirements of
/// the application and server capabilities.
const SIMULTANEOUS_REQUESTS_LIMIT: isize = 30;

/// Represents the type of HTTP request method.
///
/// This enumeration is used to specify the type of HTTP method being used in a request.
/// It supports various HTTP methods like `GET` and `PUT`. Each variant may carry
/// additional data relevant to that specific type of request. For example, the `Put`
/// variant can contain a JSON body to be sent with the request.
///
/// # Variants
///
/// - `Get`: Represents an HTTP GET request.
/// - `Put(serde_json::Value)`: Represents an HTTP PUT request with an associated JSON body.
///
/// This enum is critical for constructing and sending different types of HTTP requests,
/// allowing for flexibility and specificity in how data is communicated to a server.
#[derive(Clone)]
pub enum HttpMethod {
    Get,
    Put(serde_json::Value),
}

pub type HttpRequestResult = Result<reqwest::blocking::Response, u16>;

// A global semaphore for managing simultaneous HTTP requests.
//
// This semaphore is initialized with the maximum number of simultaneous requests defined by
// `SIMULTANEOUS_REQUESTS_LIMIT`. It is used to control access to a resource, in this case,
// the number of concurrent HTTP requests, ensuring that the limit is not exceeded.
//
// The semaphore is lazily initialized the first time it is accessed. This approach is beneficial
// for resources that require complex setup or are not used in every execution path of the program.
//
// The `Semaphore` from the `std_semaphore` crate is used to implement this functionality.
//
// The static nature of `SEMAPHORE` ensures that it is shared across all threads and parts of the
// application, maintaining a consistent and global state for request limiting.
lazy_static! {
    static ref SEMAPHORE: Semaphore = Semaphore::new(SIMULTANEOUS_REQUESTS_LIMIT);
}

fn send_http_request_single_try(
    client: &reqwest::blocking::Client,
    method: HttpMethod,
    url: &str,
    canvas_info: &CanvasCredentials,
    params: Vec<(String, String)>,
) -> HttpRequestResult {
    let request_builder = match &method {
        HttpMethod::Get => client
            .get(url)
            .bearer_auth(&canvas_info.token_canvas)
            .query(&params),
        HttpMethod::Put(body) => client
            .put(url)
            .bearer_auth(&canvas_info.token_canvas)
            .json(body),
    };

    let response = request_builder.send();

    match response {
        Ok(response) if response.status().is_success() => Ok(response),
        Ok(response) => Err(response.status().as_u16()),
        Err(_) => Err(0), // Código genérico para erros de rede ou de cliente HTTP
    }
}

pub fn send_http_request(
    client: &reqwest::blocking::Client,
    method: HttpMethod,
    url: &str,
    canvas_info: &CanvasCredentials,
    params: Vec<(String, String)>,
) -> HttpRequestResult {
    let mut attempts = 0;
    let max_attempts = 3;

    while attempts < max_attempts {
        match send_http_request_single_try(client, method.clone(), url, canvas_info, params.clone())
        {
            Ok(response) => return Ok(response),
            Err(status) if status == 403 && attempts < max_attempts - 1 => {
                attempts += 1;
                thread::sleep(Duration::from_millis(500));
            }
            Err(status) => return Err(status),
        }
    }
    Err(403) // Retorna 403 se todas as tentativas falharem
}
