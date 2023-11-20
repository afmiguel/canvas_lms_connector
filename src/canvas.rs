use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use crate::{CanvasCredentials, Course, CourseInfo};
use crate::connection::{HttpMethod, send_http_request};
use keyring::Entry;

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
    pub fn fetch_courses_with_credentials(info: &CanvasCredentials) -> CanvasResult {
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
    pub fn fetch_single_course_with_credentials(
        info: &CanvasCredentials,
        course_id: u64,
    ) -> CanvasResult {
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
    pub fn fetch_courses_<F>(f: F) -> CanvasResult
        where
            F: FnOnce(&CanvasCredentials) -> CanvasResult + Copy,
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
                    println!("Do you wish to re-register the credentials? You can find your API key in your Canvas Learning account settings. (y/n)");
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
        canvas_info: &Arc<CanvasCredentials>,
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
    pub fn load_credentials_from_file() -> Result<CanvasCredentials, String> {
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
}
