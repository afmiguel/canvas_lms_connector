use crate::connection::{send_http_request, HttpMethod};
use crate::{CanvasCredentials, Course, CourseInfo};
use std::sync::Arc;

pub enum CanvasResultCourses {
    Ok(Vec<Course>),
    ErrConnection(String),
    ErrCredentials(String),
}

pub enum CanvasResultSingleCourse {
    Ok(Course),
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
    pub fn fetch_courses_with_credentials(info: &CanvasCredentials) -> CanvasResultCourses {
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
                        return CanvasResultCourses::ErrCredentials(format!(
                            "Failed to fetch courses with status: {}",
                            response.status()
                        ));
                    }
                }
                Err(e) => {
                    return CanvasResultCourses::ErrConnection(format!(
                        "Failed to fetch courses with error: {}",
                        e
                    ));
                }
            }
        }

        CanvasResultCourses::Ok(all_courses)
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
    ) -> CanvasResultSingleCourse {
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
                        return CanvasResultSingleCourse::Ok(course);
                    } else {
                        return CanvasResultSingleCourse::ErrConnection(format!("Failed to parse course data"));
                    }
                } else {
                    CanvasResultSingleCourse::ErrConnection(format!(
                        "Failed to fetch course: HTTP Status {}",
                        response.status()
                    ))
                }
            }
            Err(e) => CanvasResultSingleCourse::ErrConnection(format!("HTTP request failed: {}", e)),
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
}
