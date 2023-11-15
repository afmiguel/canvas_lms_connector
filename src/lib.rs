//! # Canvas API Integration Library
//!
//! This library provides a range of functionalities for interacting with the Canvas Learning Management System API.
//! It supports operations such as course retrieval, student management, and handling of assignments and submissions.
//! The library uses the `reqwest` library for making HTTP requests and implements concurrency control to limit the number of simultaneous requests.
//!
//! ## Core Features
//!
//! - **Authentication and Configuration:** Load Canvas API credentials from configuration files or the system keyring.
//! - **Course Management:** Retrieve information about courses available to an authenticated user.
//! - **Student Management:** Fetch students enrolled in specific courses.
//! - **Assignments and Submissions Handling:** Retrieve and update assignments and student submissions.
//!
//! ## Usage
//!
//! To use this library, add it as a dependency in your `Cargo.toml`. Then, utilize the structures and functions
//! provided to interact with the Canvas API as needed for your application.
//!
//! ```toml
//! [dependencies]
//! canvas_lms_connector = "0.1.1"
//! ```
//!
//! After adding the dependency, you can start using the library's features in your code.
//!
//! The entry point is the function `fetch_courses`, which can be used to retrieve courses.
//! The function takes a closure as an argument, which is used to specify the type of course retrieval.
//! The closure should take a reference to `CanvasInfo` and return a `CanvasResult`.
//! The `CanvasResult` enum encapsulates the possible outcomes of the operation, including successful
//! retrieval of courses or various types of errors.
//! The `fetch_courses` function handles credential management and error handling, while the closure
//! specifies the specific course retrieval logic.
//! This approach allows for flexibility in the way courses are retrieved, while ensuring that the
//! authentication and error handling are handled in a consistent way.
//!
//! The following example demonstrates the usage of `fetch_courses` to retrieve courses using
//! credentials stored in the system keyring.
//!
//! # Examples
//!
//! ```
//! match Canvas::fetch_courses(|credential| Canvas::fetch_courses_with_credentials(credential)) {
//!     CanvasResult::Ok(courses) => println!("Courses fetched: {:?}", courses),
//!     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
//!     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
//! }
//! ```
//! or
//!  ```
//! let course_id = 123;
//! match Canvas::fetch_courses(|credential| Canvas::fetch_single_course_with_credentials(credential, course_id)) {
//!     CanvasResult::Ok(courses) => println!("Courses fetched: {:?}", courses),
//!     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
//!     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
//! }
//! ```
use chrono::{DateTime, Utc};
use keyring::Entry;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::{thread, time::Duration};
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
enum HttpMethod {
    Get,
    Put(serde_json::Value),
}

/// Type alias for the result of an HTTP request.
///
/// This type is a shorthand for the `Result` type specialized for HTTP requests made using
/// the `reqwest` crate's blocking client. It simplifies the handling of responses and errors
/// from HTTP requests.
///
/// The `Ok` variant of this type contains a `reqwest::blocking::Response`, which represents
/// the response received from the HTTP request. The `Err` variant contains a boxed error
/// (`Box<dyn std::error::Error>`), which allows for error flexibility and can represent a variety
/// of errors that might occur during an HTTP request, such as connection issues, timeout errors,
/// or other request-related errors.
type HttpRequestResult = Result<reqwest::blocking::Response, Box<dyn std::error::Error>>;

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

/// Sends an HTTP request using the specified parameters.
///
/// This function handles the logic of sending an HTTP request based on the provided method, URL,
/// and additional parameters. It utilizes the `reqwest::blocking::Client` for HTTP request execution.
/// The function supports retrying requests and managing the rate of requests using a semaphore.
///
/// # Arguments
///
/// * `client` - A reference to the `reqwest::blocking::Client` for executing HTTP requests.
/// * `method` - The HTTP method (`HttpMethod`) to be used for the request.
/// * `url` - The URL endpoint to which the request will be sent.
/// * `canvas_info` - A reference to the `CanvasInfo` containing authentication token.
/// * `params` - A vector of key-value pairs for query parameters or body content.
///
/// # Errors
///
/// Returns an error if the request fails after the maximum number of attempts or encounters
/// issues like connection errors, time-outs, etc.
///
/// # Examples
///
/// ```
/// // Example usage of send_http_request
/// let client = reqwest::blocking::Client::new();
/// let result = send_http_request(&client, HttpMethod::Get, "https://example.com", &canvas_info, params);
/// ```

fn send_http_request(
    client: &reqwest::blocking::Client,
    method: HttpMethod,
    url: &str,
    canvas_info: &CanvasInfo,
    params: Vec<(String, String)>,
) -> HttpRequestResult {
    let mut last_error: Option<Box<dyn std::error::Error>> = None;
    let max_attempts = 3;
    let mut attempts = 0;

    while attempts < max_attempts {
        attempts += 1;

        // Adquirir o semáforo imediatamente antes da tentativa
        SEMAPHORE.acquire();

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

        // Liberar o semáforo imediatamente após a tentativa
        SEMAPHORE.release();

        match response {
            Ok(response) if response.status().is_success() => return Ok(response),
            Ok(response) => {
                let status = response.status();
                last_error = Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Request failed with status: {}", status),
                )));
            }
            Err(e) => {
                // Converte reqwest::Error em um Box<dyn std::error::Error>
                last_error = Some(Box::new(e) as Box<dyn std::error::Error>);
            }
        }

        // Aguardar entre as tentativas, se ainda houver tentativas restantes
        if attempts < max_attempts {
            thread::sleep(Duration::from_millis(100));
        }
    }

    // Se todas as tentativas falharem, retorna o último erro
    Err(last_error.unwrap_or_else(|| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed after 3 attempts",
        )) as Box<dyn std::error::Error>
    }))
}

/// Represents the possible outcomes of operations interacting with the Canvas system.
///
/// This enumeration is used to encapsulate the results of various operations when interacting with the Canvas Learning Management System, such as fetching courses or handling credentials. It differentiates between successful outcomes and various types of errors that might occur.
///
/// # Variants
///
/// - `Ok(Vec<Course>)`: Indicates a successful operation, returning a list of `Course` objects.
/// - `ErrConnection(String)`: Represents a failure due to connection issues, with an accompanying error message.
/// - `ErrCredentials(String)`: Indicates an error related to authentication or credentials, with a detailed message.
///
/// # Examples
///
/// ```
/// // Example of using CanvasResult to handle the outcome of a course fetching operation
/// match fetch_courses_with_credentials(&canvas_info) {
///     CanvasResult::Ok(courses) => println!("Courses fetched successfully"),
///     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
///     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
/// }
/// ```
pub enum CanvasResult {
    Ok(Vec<Course>),
    ErrConnection(String),
    ErrCredentials(String),
}

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
pub struct CanvasInfo {
    pub url_canvas: String,
    pub token_canvas: String,
}

/// Represents the main interface for interacting with the Canvas Learning Management System (LMS).
///
/// This structure provides methods for performing various operations related to the Canvas LMS,
/// such as fetching courses or handling authentication. It acts as a central point for accessing
/// the functionalities offered by the Canvas API.
///
/// # Examples
///
/// ```
/// // Example of using the Canvas struct to fetch courses
/// let canvas = Canvas { /* fields initialization */ };
/// match canvas.fetch_courses_with_credentials(&canvas_info) {
///     Ok(courses) => println!("Courses retrieved: {:?}", courses),
///     Err(e) => eprintln!("Error fetching courses: {:?}", e),
/// }
/// ```
pub struct Canvas {
    // info: Arc<CanvasInfo>,
}

/// Implementation of Canvas struct functionalities.
///
/// Provides methods to interact with the Canvas Learning Management System (LMS),
/// encapsulating the logic for operations such as fetching courses, managing credentials,
/// and other interactions with the Canvas API.
///
/// # Methods
///
/// - `fetch_courses_with_credentials`: Fetches courses using specific Canvas credentials.
/// - `fetch_courses`: Retrieves courses using stored or system-provided credentials.
/// - `load_credentials_from_file`: Loads Canvas credentials from a configuration file.
/// - `load_credentials_from_system`: Loads Canvas credentials stored in the system keyring.
///
/// Each method focuses on a specific aspect of Canvas LMS interaction, ensuring ease of use
/// in various application contexts.
impl Canvas {
    /// Fetches a list of courses from the Canvas API using the provided credentials.
    ///
    /// This function attempts to retrieve all courses accessible with the given Canvas credentials.
    /// It utilizes the Canvas API endpoint to gather course information, authenticating the request
    /// with the credentials supplied in the `CanvasInfo` structure.
    ///
    /// # Arguments
    ///
    /// * `info` - A reference to `CanvasInfo` containing the URL and API token for Canvas API access.
    ///
    /// # Returns
    ///
    /// A `CanvasResult` enum, which can be either:
    /// - `CanvasResult::Ok(Vec<Course>)` on successful retrieval of courses.
    /// - `CanvasResult::ErrConnection(String)` for connection-related errors.
    /// - `CanvasResult::ErrCredentials(String)` for authentication failures.
    ///
    /// # Examples
    ///
    /// ```
    /// let canvas_info = CanvasInfo { /* fields initialization */ };
    /// match fetch_courses_with_credentials(&canvas_info) {
    ///     CanvasResult::Ok(courses) => println!("Courses fetched: {:?}", courses),
    ///     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
    ///     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
    /// }
    /// ```
    pub fn fetch_courses_with_credentials(info: &CanvasInfo) -> CanvasResult {
        let canvas_info_arc = Arc::new((*info).clone());

        let url = format!("{}/courses", info.url_canvas);
        let mut all_courses = Vec::new();
        let mut page = 1;
        let client = &reqwest::blocking::Client::new();

        loop {
            let params = vec![
                (
                    "enrollment_role".to_string(),
                    "TeacherEnrollment".to_string(),
                ),
                ("page".to_string(), page.to_string()),
                ("per_page".to_string(), "100".to_string()),
            ];

            match send_http_request(client, HttpMethod::Get, &url, &info, params) {
                Ok(response) => {
                    if response.status().is_success() {
                        let courses: Vec<serde_json::Value> = response.json().unwrap();
                        if courses.is_empty() {
                            break; // Exit loop if no courses are returned
                        }
                        all_courses.extend(courses.iter().filter_map(|course| {
                            Canvas::convert_json_to_course(&canvas_info_arc, course)
                            // Send the Arc to the function
                        }));
                        page += 1; // Increnent page number
                    } else {
                        return CanvasResult::ErrCredentials(format!(
                            "Failed to fetch courses with status: {}",
                            response.status()
                        ));
                    }
                }
                Err(e) => {
                    return CanvasResult::ErrConnection(format!(
                        "Failed to fetch courses with error: {}",
                        e
                    ));
                }
            }
        }

        CanvasResult::Ok(all_courses)
    }

    /// Fetches details of a specific course from the Canvas API using provided credentials.
    ///
    /// This function retrieves information about a single course, identified by `course_id`, using the credentials
    /// provided in `info`. It communicates with the Canvas API and returns the course data in a `CanvasResult`.
    ///
    /// # Arguments
    ///
    /// * `info` - A reference to `CanvasInfo` containing the URL and API token for Canvas API access.
    /// * `course_id` - The unique identifier for the course to be fetched.
    ///
    /// # Returns
    ///
    /// A `CanvasResult` enum, which is either:
    /// - `CanvasResult::Ok(Vec<Course>)` on successful retrieval of the course.
    /// - `CanvasResult::ErrConnection(String)` for connection-related errors.
    /// - `CanvasResult::ErrCredentials(String)` for authentication failures.
    ///
    /// # Examples
    ///
    /// ```
    /// let canvas_info = CanvasInfo { /* fields initialization */ };
    /// let course_id = 123;
    /// match fetch_single_course_with_credentials(&canvas_info, course_id) {
    ///     CanvasResult::Ok(course) => println!("Course fetched: {:?}", course),
    ///     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
    ///     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
    /// }
    /// ```
    pub fn fetch_single_course_with_credentials(info: &CanvasInfo, course_id: u64) -> CanvasResult {
        let canvas_info_arc = Arc::new((*info).clone());
        let url = format!("{}/courses/{}", info.url_canvas, course_id);

        match send_http_request(
            &reqwest::blocking::Client::new(),
            HttpMethod::Get,
            &url,
            info,
            Vec::new(), // No additional parameters for this request
        ) {
            Ok(response) => {
                if response.status().is_success() {
                    let course: serde_json::Value = response.json().unwrap();
                    if let Some(course) = Canvas::convert_json_to_course(&canvas_info_arc, &course)
                    {
                        return CanvasResult::Ok(vec![course]);
                    } else {
                        return CanvasResult::ErrConnection(format!("Failed to parse course data"));
                    }
                } else {
                    CanvasResult::ErrConnection(format!(
                        "Failed to fetch course: HTTP Status {}",
                        response.status()
                    ))
                }
            }
            Err(e) => CanvasResult::ErrConnection(format!("HTTP request failed: {}", e)),
        }
    }

    /// Fetches a list of courses using a provided function for specific fetching logic.
    ///
    /// This higher-order function takes another function `f` as an argument, which contains the logic for fetching courses.
    /// It allows for customization in the way courses are retrieved, while `fetch_courses` itself handles credential management and error handling.
    ///
    /// # Type Parameters
    ///
    /// * `F`: A function or closure that takes a reference to `CanvasInfo` and returns `CanvasResult`.
    ///
    /// # Arguments
    ///
    /// * `f` - The function or closure that will be called with the loaded `CanvasInfo` to fetch courses.
    ///
    /// # Returns
    ///
    /// A `CanvasResult` enum, either:
    /// - `CanvasResult::Ok(Vec<Course>)` on successful retrieval of courses.
    /// - `CanvasResult::ErrConnection(String)` for connection-related errors.
    /// - `CanvasResult::ErrCredentials(String)` for authentication failures.
    ///
    /// # Examples
    ///
    /// ```
    /// match Canvas::fetch_courses(|credential| Canvas::fetch_courses_with_credentials(credential)) {
    ///     CanvasResult::Ok(courses) => println!("Courses fetched: {:?}", courses),
    ///     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
    ///     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
    /// }
    /// ```
    /// or
    ///  ```
    /// let course_id = 123;
    /// match Canvas::fetch_courses(|credential| Canvas::fetch_single_course_with_credentials(credential, course_id)) {
    ///     CanvasResult::Ok(courses) => println!("Courses fetched: {:?}", courses),
    ///     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
    ///     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
    /// }
    /// ```
    pub fn fetch_courses<F>(f: F) -> CanvasResult
    where
        F: FnOnce(&CanvasInfo) -> CanvasResult + Copy,
    {
        let app_name = env!("CARGO_PKG_NAME");
        match Canvas::load_credentials_from_file() {
            Ok(credentials_ok) => {
                return f(&credentials_ok);
            }
            Err(_) => {
                loop {
                    match Canvas::load_credentials_from_system() {
                        Ok(credentials_ok) => match f(&credentials_ok) {
                            CanvasResult::Ok(list_of_courses) => {
                                return CanvasResult::Ok(list_of_courses);
                            }
                            CanvasResult::ErrConnection(e) => {
                                return CanvasResult::ErrConnection(
                                    format!("Connection error: {}", e).to_string(),
                                );
                            }
                            CanvasResult::ErrCredentials(e) => {
                                println!("Incorrect credentials: {}", e);
                            }
                        },
                        Err(_) => {
                            println!("Error obtaining credentials");
                        }
                    }
                    println!("Do you wish to re-register the credentials? (y/n)");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).unwrap();
                    if input.trim().to_uppercase() != "Y" {
                        return CanvasResult::ErrCredentials("Incorrect credentials".to_string());
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
                        return CanvasResult::ErrCredentials(
                            format!("Error saving URL: {}", e).to_string(),
                        );
                    }
                    if let Err(e) = Entry::new(app_name, "TOKEN_CANVAS")
                        .unwrap()
                        .set_password(token.as_str())
                    {
                        return CanvasResult::ErrCredentials(
                            format!("Error saving token: {}", e).to_string(),
                        );
                    }
                }
            }
        }
    }

    /// Converts a JSON object to a `Course` structure.
    ///
    /// This function takes a JSON representation of a course, typically obtained from the Canvas API,
    /// and transforms it into a `Course` object. It extracts necessary information such as the course's
    /// ID, name, and code from the JSON object and associates it with the provided `CanvasInfo`.
    ///
    /// # Arguments
    ///
    /// * `canvas_info` - A shared reference to the `CanvasInfo` containing Canvas API credentials.
    /// * `course` - A reference to the JSON object representing a course as returned by the Canvas API.
    ///
    /// # Returns
    ///
    /// Returns `Some(Course)` if the conversion is successful, or `None` if the required fields are not present
    /// in the JSON object or cannot be properly parsed.
    ///
    /// # Examples
    ///
    /// ```
    /// let canvas_info = Arc::new(CanvasInfo { /* fields initialization */ });
    /// let course_json = serde_json::json!({ /* course data in JSON format */ });
    /// if let Some(course) = convert_json_to_course(&canvas_info, &course_json) {
    ///     println!("Course converted: {:?}", course);
    /// }
    /// ```
    fn convert_json_to_course(
        canvas_info: &Arc<CanvasInfo>,
        course: &serde_json::Value,
    ) -> Option<Course> {
        let id = course["id"].as_u64()?;
        let name = course["name"].as_str().map(String::from)?;
        let course_code = course["course_code"].as_str().map(String::from)?;
        Some(Course {
            info: Arc::new(CourseInfo {
                id,
                name,
                course_code,
                canvas_info: Arc::clone(canvas_info),
            }),
        })
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
    pub fn load_credentials_from_file() -> Result<CanvasInfo, String> {
        if let Some(mut home_config_buffer) = dirs::home_dir() {
            home_config_buffer.push("Downloads");
            home_config_buffer.push("config.json");
            if let Some(config_path) = home_config_buffer.to_str() {
                if let Ok(file) = File::open(config_path) {
                    println!("Configuration file found: {}", config_path);
                    let reader = BufReader::new(file);
                    let config: Result<CanvasInfo, serde_json::Error> =
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
    pub fn load_credentials_from_system() -> Result<CanvasInfo, String> {
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
                                    return Ok(CanvasInfo {
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
}

/// Contains detailed information about a course in the Canvas system.
///
/// This structure is used to store and manage specific data related to a Canvas course. It includes
/// essential information such as the course's unique identifier, name, and course code. Additionally,
/// it holds a reference to `CanvasInfo`, providing the necessary context for API requests related
/// to the course.
///
/// # Fields
///
/// - `id`: The unique identifier for the course in the Canvas system.
/// - `name`: The name of the course.
/// - `course_code`: The course code or short identifier.
/// - `canvas_info`: A shared reference to the `CanvasInfo` containing Canvas API credentials and URL.
///
/// The structure is fundamental for representing a course and its related operations within the Canvas API context.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CourseInfo {
    pub id: u64,
    pub name: String,
    pub course_code: String,
    #[serde(skip)]
    pub canvas_info: Arc<CanvasInfo>,
}

/// Represents a course within the Canvas Learning Management System.
///
/// This structure encapsulates data about a specific course, primarily through the `CourseInfo` struct,
/// which contains detailed attributes like the course's ID, name, and code. The `Course` struct serves as
/// a high-level representation of a course in the Canvas system, facilitating the access and manipulation
/// of course-related information in various operations and API interactions.
///
/// # Fields
///
/// - `info`: A shared reference (`Arc`) to a `CourseInfo` instance containing detailed information about
///   the course.
///
/// The `Course` struct is a key component in applications interacting with the Canvas API, providing a
/// convenient and unified way to handle course-related data.
#[derive(Clone)]
pub struct Course {
    pub info: Arc<CourseInfo>,
}

/// Implementation block for the `Course` struct.
///
/// Provides a set of methods for interacting with specific course-related data and functionalities
/// in the Canvas Learning Management System. This includes fetching students and assignments associated
/// with a course, as well as handling submissions and other course-specific operations.
///
/// The methods utilize the `CourseInfo` contained within the `Course` struct to make relevant API calls,
/// ensuring that the interactions are specific to the particular course instance.
///
/// # Methods
///
/// - `fetch_students`: Retrieves a list of students enrolled in the course.
/// - `fetch_assignments`: Fetches assignments associated with the course.
/// - `update_assignment_score`: Updates the score for a specific assignment submission.
///
/// These methods are essential for applications that require detailed interaction with courses in the
/// Canvas system, such as retrieving student lists, managing assignments, and processing grades.
impl Course {
    /// Fetches a list of students enrolled in the course.
    ///
    /// This method queries the Canvas API to retrieve students associated with the course
    /// represented by this `Course` instance. It makes use of the course's ID and the Canvas API
    /// credentials stored in `CourseInfo` to make the appropriate API call.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(Vec<Student>)` containing a list of students if the fetch is successful.
    /// - `Err(Box<dyn std::error::Error>)` encapsulating any errors encountered during the API request,
    ///   such as network issues, data parsing errors, or API access problems.
    ///
    /// # Examples
    ///
    /// ```
    /// let course = Course { /* fields initialization - see Canvas::fetch_courses */ };
    /// match course.fetch_students() {
    ///     Ok(students) => println!("Students: {:?}", students),
    ///     Err(e) => eprintln!("Error fetching students: {:?}", e),
    /// }
    /// ```
    pub fn fetch_students(&self) -> Result<Vec<Student>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/courses/{}/users",
            &self.info.canvas_info.url_canvas, self.info.id
        );

        let mut all_students = Vec::new();
        let mut page = 1;
        let client = &reqwest::blocking::Client::new();

        loop {
            let params = vec![
                ("enrollment_type[]", "student".to_string()),
                ("include[]", "email".to_string()),
                ("per_page", "150".to_string()),
                ("page", page.to_string()),
            ];

            // Convertendo (&str, String) para (String, String)
            let converted_params: Vec<(String, String)> = params
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect();

            // Passando HttpMethod::Get ao invés de "GET"
            let response = send_http_request(
                client,
                HttpMethod::Get, // Supondo que HttpMethod::Get é um enum definido em algum lugar
                &url,
                &self.info.canvas_info,
                converted_params, // Passando o Vec<(String, String)> diretamente
            )?;

            // Verifica o código de status e interrompe o loop ou processa os dados
            if response.status() == reqwest::StatusCode::OK {
                let students_page: Vec<serde_json::Value> = response.json()?;
                if students_page.is_empty() {
                    break; // Sai do loop se não há mais estudantes
                }
                all_students.extend(
                    students_page.into_iter().filter_map(|student| {
                        Course::convert_json_to_student(&self.info, &student)
                    }),
                );
                page += 1; // Incrementa o número da página para a próxima iteração
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to fetch students with status: {}",
                        response.status()
                    ),
                )));
            }
        }

        Ok(all_students)
    }

    /// Converts a JSON object to a `Student` structure.
    ///
    /// This function takes a JSON representation of a student, typically obtained from the Canvas API,
    /// and transforms it into a `Student` object. It extracts necessary information such as the student's
    /// ID, name, and email from the JSON object and associates it with the provided `CourseInfo`.
    ///
    /// # Arguments
    ///
    /// * `course_info` - A shared reference to the `CourseInfo` containing essential details about the course.
    /// * `student` - A reference to the JSON object representing a student as returned by the Canvas API.
    ///
    /// # Returns
    ///
    /// Returns `Some(Student)` if the conversion is successful, or `None` if the required fields are not present
    /// in the JSON object or cannot be properly parsed.
    ///
    /// # Examples
    ///
    /// ```
    /// let course_info = Arc::new(CourseInfo { /* fields initialization */ });
    /// let student_json = serde_json::json!({ /* student data in JSON format */ });
    /// if let Some(student) = convert_json_to_student(&course_info, &student_json) {
    ///     println!("Student converted: {:?}", student);
    /// }
    /// ```
    fn convert_json_to_student(
        course_info: &Arc<CourseInfo>,
        student: &serde_json::Value,
    ) -> Option<Student> {
        let id = student["id"].as_u64()?;
        let name = student["name"].as_str().map(String::from)?;
        let email = student["email"].as_str().map(String::from)?;
        Some(Student {
            info: Arc::new(StudentInfo {
                id,
                name,
                email,
                course_info: Arc::clone(course_info),
            }),
        })
    }

    /// Retrieves a list of assignments for the course.
    ///
    /// This method communicates with the Canvas API to obtain all assignments associated with the
    /// course represented by this `Course` instance. It leverages the course ID and Canvas API credentials
    /// stored in `CourseInfo` to perform authenticated requests.
    ///
    /// The method handles the pagination of the Canvas API response, ensuring the collection of all assignments.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(Vec<Assignment>)` containing a vector of assignments if the request is successful.
    /// - `Err(Box<dyn std::error::Error>)` encapsulating any encountered errors during the API call,
    ///   such as network issues, parsing errors, or API access problems.
    ///
    /// # Examples
    ///
    /// ```
    /// let course = Course { /* fields initialization - see Canvas::fetch_courses */ };
    /// match course.fetch_assignments() {
    ///     Ok(assignments) => println!("Assignments: {:?}", assignments),
    ///     Err(e) => eprintln!("Error fetching assignments: {:?}", e),
    /// }
    /// ```
    pub fn fetch_assignments(&self) -> Result<Vec<Assignment>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/courses/{}/assignments",
            self.info.canvas_info.url_canvas, self.info.id
        );

        let mut all_assignments = Vec::new();
        let mut page = 1;
        let client = &reqwest::blocking::Client::new();
        loop {
            // Construindo os parâmetros da requisição
            let params = vec![
                ("page".to_string(), page.to_string()),
                ("per_page".to_string(), "100".to_string()),
            ];

            // Enviando a requisição GET
            let response = send_http_request(
                client,
                HttpMethod::Get,
                &url,
                &self.info.canvas_info,
                params,
            )?;

            // Verifique o sucesso da resposta e processe o JSON
            if response.status().is_success() {
                let assignments: Vec<serde_json::Value> = response.json()?;
                if assignments.is_empty() {
                    break; // Sai do loop se não há mais cursos
                }
                let assignments = assignments
                    .iter()
                    .filter_map(|assignment| {
                        Course::convert_json_to_assignment(&self.info, assignment)
                    })
                    .collect::<Vec<_>>();
                all_assignments.extend(assignments);
                page += 1; // Incrementa o número da página para a próxima iteração
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to fetch assignments with status: {}",
                        response.status()
                    ),
                )));
            }
        }
        Ok(all_assignments)
    }

    /// Converts a JSON object into an `Assignment` structure.
    ///
    /// This function parses a JSON representation of an assignment, typically retrieved from the Canvas API,
    /// and converts it into an `Assignment` object. It extracts key details such as the assignment's ID, name,
    /// and description from the JSON object and links it with the provided `CourseInfo`.
    ///
    /// # Arguments
    ///
    /// * `course_info` - A shared reference to the `CourseInfo` containing relevant course details.
    /// * `assignment` - A reference to the JSON object representing an assignment from the Canvas API.
    ///
    /// # Returns
    ///
    /// Returns `Some(Assignment)` if the conversion is successful, or `None` if essential data is missing
    /// or cannot be correctly parsed.
    ///
    /// # Examples
    ///
    /// ```
    /// let course_info = Arc::new(CourseInfo { /* fields initialization */ });
    /// let assignment_json = serde_json::json!({ /* assignment data in JSON format */ });
    /// if let Some(assignment) = convert_json_to_assignment(&course_info, &assignment_json) {
    ///     println!("Assignment converted: {:?}", assignment);
    /// }
    /// ```
    fn convert_json_to_assignment(
        course_info: &Arc<CourseInfo>,
        assignment: &serde_json::Value,
    ) -> Option<Assignment> {
        let id = assignment["id"].as_u64()?;
        let name = assignment["name"].as_str().map(String::from)?;
        let description = assignment["description"].as_str().map(String::from);
        Some(Assignment {
            info: Arc::new(AssignmentInfo {
                id,
                name,
                description,
                course_info: Arc::clone(course_info),
            }),
        })
    }

    /// Updates the score for a specific assignment submission.
    ///
    /// This method allows updating the score of a student's submission for a particular assignment.
    /// It sends an HTTP PUT request to the Canvas API with the new score. The function handles the
    /// request construction and execution, including authentication using the provided `CanvasInfo`.
    ///
    /// # Arguments
    ///
    /// * `client` - A reference to the `reqwest::blocking::Client` used for making HTTP requests.
    /// * `assignment_id` - The unique identifier of the assignment for which the score is being updated.
    /// * `student_id` - The unique identifier of the student whose submission score is being updated.
    /// * `new_score` - An `Option<f64>` representing the new score to be set. A `None` value indicates
    ///   that the existing score should be cleared.
    ///
    /// # Returns
    ///
    /// Returns a `Result` type:
    /// - `Ok(())` on successful score update.
    /// - `Err(Box<dyn std::error::Error>)` encapsulating any encountered errors during the API call,
    ///   such as network issues, parsing errors, or API access problems.
    ///
    /// # Examples
    ///
    /// ```
    /// let client = reqwest::blocking::Client::new();
    /// let assignment_id = 123;
    /// let student_id = 456;
    /// let new_score = Some(95.0);
    /// match course.update_assignment_score(&client, assignment_id, student_id, new_score) {
    ///     Ok(_) => println!("Score updated successfully"),
    ///     Err(e) => eprintln!("Error updating score: {:?}", e),
    /// }
    /// ```
    pub fn update_assignment_score(
        &self,
        client: &reqwest::blocking::Client,
        assignment_id: u64,
        student_id: u64,
        new_score: Option<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "{}/courses/{}/assignments/{}/submissions/{}",
            self.info.canvas_info.url_canvas, self.info.id, assignment_id, student_id,
        );

        let body;
        if let Some(new_score) = new_score {
            body = serde_json::json!({
                "submission": {
                    "posted_grade": new_score
                }
            });
        } else {
            body = serde_json::json!({
                "submission": {
                    "posted_grade": ""
                }
            });
        }

        let response = send_http_request(
            client,
            HttpMethod::Put(body), // Use HttpMethod::Put enum variant
            &url,
            &self.info.canvas_info,
            Vec::new(), // PUT request does not need params
        )?;

        match response.status().is_success() {
            true => Ok(()),
            false => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to update score with status: {}", response.status()),
            ))),
        }
    }
}

/// Contains detailed information about a student in the Canvas system.
///
/// This structure is used to store and handle specific data related to a student within a Canvas course.
/// It includes vital information such as the student's unique identifier, name, and email address.
/// Additionally, it holds a reference to `CourseInfo`, providing context for API requests related
/// to the student within the specific course.
///
/// # Fields
///
/// - `id`: The unique identifier of the student in the Canvas system.
/// - `name`: The full name of the student.
/// - `email`: The email address of the student.
/// - `course_info`: A shared reference (`Arc`) to `CourseInfo` containing Canvas API credentials and course details.
///
/// This structure is fundamental for representing a student and performing various operations related
/// to student data within the Canvas API context.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StudentInfo {
    pub id: u64,
    pub name: String,
    pub email: String,
    #[serde(skip)]
    pub course_info: Arc<CourseInfo>,
}

/// Represents a student within the Canvas Learning Management System.
///
/// This structure encapsulates information about a student enrolled in a course. It primarily contains
/// `StudentInfo`, which holds details like the student's ID, name, and email, along with course-related
/// information. The `Student` struct serves as a high-level representation of a student in Canvas,
/// enabling access to and manipulation of student-related data in various operations and API interactions.
///
/// # Fields
///
/// - `info`: A shared reference (`Arc`) to a `StudentInfo` instance containing detailed information about
///   the student.
///
/// The `Student` struct is a key component in applications interacting with the Canvas API, providing a
/// convenient and unified way to handle student-related data.
pub struct Student {
    pub info: Arc<StudentInfo>,
}

/// Implementation block for the `Student` struct.
///
/// Provides methods to interact with student-specific data and functionalities in the Canvas Learning
/// Management System. This includes fetching submissions for assignments, as well as retrieving and
/// updating submissions and other student-related operations.
///
/// The methods utilize the `StudentInfo` contained within the `Student` struct to make relevant API calls,
/// ensuring that the interactions are specific to the particular student instance.
///
/// # Methods
///
/// - `fetch_submissions_for_assignments`: Retrieves submissions for a set of assignments for the student.
/// - `update_assignment_score`: Updates the score for a specific assignment submission for the student.
///
/// These methods are crucial for applications that require detailed interaction with student data in the
/// Canvas system, such as tracking assignment submissions and managing academic records.
impl Student {
    /// Fetches submissions for a given set of assignments for the student.
    ///
    /// This method queries the Canvas API to obtain submissions made by the student for specified assignments.
    /// It uses the student's ID and the course's Canvas API credentials for authenticated requests. The function
    /// is designed to handle multiple assignment IDs, making it versatile for fetching submissions across different
    /// assignments.
    ///
    /// # Type Parameters
    ///
    /// - `F`: The type of the closure passed for additional interactions. It must be a function that takes no
    ///   parameters and returns nothing (`Fn()`).
    ///
    /// # Arguments
    ///
    /// * `client` - A reference to the `reqwest::blocking::Client` for executing HTTP requests.
    /// * `assignment_ids` - A slice of assignment IDs for which submissions need to be fetched.
    /// * `interaction` - A closure that can be called for additional side effects or interactions.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(Vec<Submission>)` containing a list of submissions if the request is successful.
    /// - `Err(Box<dyn std::error::Error>)` encapsulating any encountered errors during the API call,
    ///   such as network issues, parsing errors, or API access problems.
    ///
    /// # Examples
    ///
    /// ```
    /// let client = reqwest::blocking::Client::new();
    /// let assignment_ids = vec![123, 456];
    /// let interaction = || { /* interaction logic here */ };
    /// match student.fetch_submissions_for_assignments(&client, &assignment_ids, interaction) {
    ///     Ok(submissions) => println!("Submissions: {:?}", submissions),
    ///     Err(e) => eprintln!("Error fetching submissions: {:?}", e),
    /// }
    /// ```
    pub fn fetch_submissions_for_assignments<F>(
        &self,
        client: &reqwest::blocking::Client,
        assignment_ids: &[u64],
        interaction: F,
    ) -> Result<Vec<Submission>, Box<dyn std::error::Error>>
    where
        F: Fn(),
    {
        let canvas_base_url = &self.info.course_info.canvas_info.url_canvas;
        let mut submissions = Vec::new();

        for &assignment_id in assignment_ids {
            // update_carrossel();
            let url = format!(
                "{}/courses/{}/assignments/{}/submissions/{}",
                canvas_base_url, self.info.course_info.id, assignment_id, self.info.id
            );

            // Não são necessários parâmetros adicionais para esta chamada de API específica
            let params = Vec::new(); // Sem parâmetros adicionais para a requisição GET

            let response = send_http_request(
                client,
                HttpMethod::Get,                    // Método GET
                &url,                               // URL da API
                &self.info.course_info.canvas_info, // Token de acesso
                params,                             // Parâmetros da requisição
            )?;
            interaction();

            if response.status().is_success() {
                let submission: Submission = response.json()?; // Deserializar a resposta JSON para um objeto Submission
                submissions.push(submission);
            } else {
                let error_message = response.text()?;
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to fetch submissions with error: {}", error_message),
                )));
            }
        }
        Ok(submissions)
    }

    /// Retrieves assignments and their latest submissions for the student.
    ///
    /// This method fetches assignments associated with the student and their corresponding latest submissions.
    /// It leverages a collection of `Assignment` objects, using their IDs to query the Canvas API for submissions
    /// made by the student. The function provides a comprehensive view of both assignment information and submission
    /// details.
    ///
    /// # Type Parameters
    ///
    /// - `F`: The type of the closure for additional interactions, following the `Fn()` trait.
    ///
    /// # Arguments
    ///
    /// * `client` - A reference to the `reqwest::blocking::Client` used for HTTP requests.
    /// * `assignments` - An `Arc` containing a vector of `Assignment` objects to process.
    /// * `interaction` - A closure for additional operations executed during processing.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(HashMap<u64, (Arc<AssignmentInfo>, Option<Submission>)>)`: A hash map where each key is an
    ///   assignment ID, and the value is a tuple containing the `AssignmentInfo` and an `Option` for the
    ///   latest `Submission`.
    /// - `Err(Box<dyn std::error::Error>)`: Encapsulates any errors encountered during the API calls or
    ///   data processing.
    ///
    /// # Examples
    ///
    /// ```
    /// let client = reqwest::blocking::Client::new();
    /// let assignments = Arc::new(vec![/* Assignment objects */]);
    /// let interaction = || { /* interaction logic here */ };
    /// match student.fetch_assignments_and_latest_submissions(&client, assignments, interaction) {
    ///     Ok(data) => println!("Assignments and submissions: {:?}", data),
    ///     Err(e) => eprintln!("Error: {:?}", e),
    /// }
    /// ```
    pub fn fetch_assignments_and_latest_submissions<F>(
        &self,
        client: &reqwest::blocking::Client,
        assignments: Arc<Vec<Assignment>>,
        interaction: F,
    ) -> Result<HashMap<u64, (Arc<AssignmentInfo>, Option<Submission>)>, Box<dyn std::error::Error>>
    where
        F: Fn(),
    {
        let assignment_ids: Vec<u64> = assignments
            .iter()
            .map(|assignment| assignment.info.id)
            .collect();

        let submissions =
            self.fetch_submissions_for_assignments(client, &assignment_ids, interaction)?;

        let mut association: HashMap<u64, (Arc<AssignmentInfo>, Option<Submission>)> =
            HashMap::new();
        for assignment in assignments.iter() {
            let relevant_submissions: Vec<_> = submissions
                .iter()
                .filter(|submission| submission.assignment_id == assignment.info.id)
                .collect();

            let latest_submission = relevant_submissions
                .into_iter()
                .max_by_key(|submission| submission.submitted_at.clone());

            association.insert(
                assignment.info.id,
                (Arc::clone(&assignment.info), latest_submission.cloned()),
            );
        }
        // println!("Z");
        Ok(association)
    }
}

/// Represents a student's submission for an assignment in the Canvas Learning Management System.
///
/// This structure contains detailed information about a specific submission made by a student for an assignment.
/// It includes the submission's unique identifier, the assignment ID it belongs to, the score (if graded),
/// and the timestamp when the submission was made. Additionally, it holds a reference to the `StudentInfo` of
/// the student who made the submission, linking it directly to the student's data.
///
/// # Fields
///
/// - `id`: The unique identifier of the submission in the Canvas system.
/// - `assignment_id`: The ID of the assignment to which this submission corresponds.
/// - `score`: An optional field containing the score of the submission, if graded.
/// - `submitted_at`: An optional field indicating the date and time of submission.
/// - `student`: A shared reference (`Arc`) to `StudentInfo`, linking the submission to the student.
///
/// The structure plays a vital role in the digital workflow of assignments, enabling effective tracking
/// and assessment of student performance in the Canvas environment.
///
/// See also: fetch_submissions_for_assignments, fetch_assignments_and_latest_submissions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Submission {
    pub id: u64,
    pub assignment_id: u64,
    pub score: Option<f64>,
    pub submitted_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub student: Arc<StudentInfo>,
}

/// Contains detailed information about an assignment in the Canvas system.
///
/// This structure is used to store and manage data specific to an assignment within a Canvas course.
/// It includes key details such as the assignment's unique identifier, its name, and a description (if provided).
/// Additionally, the structure holds a reference to `CourseInfo`, linking the assignment to its respective
/// course context and providing necessary information for API interactions.
///
/// # Fields
///
/// - `id`: The unique identifier of the assignment in the Canvas system.
/// - `name`: The name of the assignment.
/// - `description`: An optional description of the assignment.
/// - `course_info`: A shared reference (`Arc`) to `CourseInfo` containing course-specific details and API credentials.
///
/// The `AssignmentInfo` structure is central to operations involving assignment data in the Canvas LMS, such as
/// fetching, updating, and managing assignments and their related activities.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AssignmentInfo {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    #[serde(skip)]
    pub course_info: Arc<CourseInfo>,
}

/// Represents an assignment within the Canvas Learning Management System.
///
/// This structure encapsulates the details of a specific assignment in a Canvas course. It primarily
/// consists of `AssignmentInfo`, which contains essential information like the assignment's ID, name,
/// and optional description. The `Assignment` struct serves as a high-level representation of an assignment
/// in Canvas, facilitating the access to and manipulation of assignment-related data in various operations
/// and API interactions.
///
/// # Fields
///
/// - `info`: A shared reference (`Arc`) to an `AssignmentInfo` instance containing detailed information
///   about the assignment.
///
/// The `Assignment` struct is a key component in applications interacting with the Canvas API, providing a
/// convenient and unified way to handle assignment-related data.
#[derive(Debug, Clone)]
pub struct Assignment {
    pub info: Arc<AssignmentInfo>,
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn it_works() {}
}
