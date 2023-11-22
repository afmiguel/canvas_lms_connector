use crate::connection::{send_http_request, HttpMethod};
use crate::{CanvasCredentials, Course, CourseInfo};
use std::sync::Arc;

/// Enum to represent the result of fetching multiple courses.
///
/// This enum provides a structured way to handle the outcomes of attempting to fetch a list of courses
/// from the Canvas LMS, distinguishing between successful retrieval, connection errors, and credential errors.
pub enum CanvasResultCourses {
    Ok(Vec<Course>),           // Success case with a vector of Course objects.
    ErrConnection(String),     // Connection error with a descriptive message.
    ErrCredentials(String),    // Credential error with a descriptive message.
}

/// Enum to represent the result of fetching a single course.
///
/// Similar to `CanvasResultCourses`, but tailored for scenarios where only a single course is being fetched.
/// Distinguishes between success, connection errors, and credential errors.
pub enum CanvasResultSingleCourse {
    Ok(Course),                // Success case with a single Course object.
    ErrConnection(String),     // Connection error with a descriptive message.
    ErrCredentials(String),    // Credential error with a descriptive message.
}


/// Main interface for interacting with the Canvas LMS.
///
/// `Canvas` struct is designed as a centralized point for accessing Canvas LMS functionalities.
/// It enables operations like fetching courses and handling authentication, encapsulating the logic
/// for Canvas API interactions.
///
/// Example:
/// ```
/// let canvas_credentials = CanvasCredentials { /* initialization */ };
/// let canvas = Canvas { /* fields initialization */ };
/// match canvas.fetch_courses_with_credentials(&canvas_credentials) {
///     CanvasResultCourses::Ok(courses) => println!("Courses: {:?}", courses),
///     CanvasResultCourses::ErrConnection(err) => eprintln!("Connection error: {}", err),
///     CanvasResultCourses::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
/// }
/// ```
pub struct Canvas {
    // info: Arc<CanvasInfo>,
}

/// Implementation block for the `Canvas` struct.
///
/// This section provides various methods to interact with the Canvas LMS, encapsulating the logic
/// necessary for operations like fetching course lists, retrieving individual course details,
/// managing Canvas credentials, and other API interactions specific to the Canvas system.
///
/// The methods are designed to streamline the process of communicating with the Canvas API,
/// handling authentication, data retrieval, and error management.
///
/// # Methods
///
/// - `fetch_courses_with_credentials`: Fetches a list of courses using specific Canvas credentials.
///   It's useful when you have multiple Canvas accounts or need to access courses under different credentials.
///
/// - `fetch_single_course_with_credentials`: Retrieves detailed information about a specific course,
///   identified by its ID, using the provided Canvas credentials. This method is particularly useful for
///   applications or services that need to focus on a single course at a time, such as a course management
///   dashboard or a student information system.
///
/// - `convert_json_to_course`: A utility function within the `Canvas` context, used by other methods to
///   transform JSON data received from the Canvas API into a structured `Course` object. This function
///   encapsulates the parsing logic, ensuring consistent conversion across different parts of the application.
///
/// Each of these methods is designed to target a specific aspect of Canvas LMS interaction, ensuring that
/// the `Canvas` struct can be used flexibly in various application contexts.
impl Canvas {
    /// Fetches a list of courses using provided Canvas credentials.
    ///
    /// Communicates with the Canvas API to retrieve accessible courses. Requires valid Canvas API credentials.
    /// Handles pagination to ensure all courses are fetched.
    ///
    /// Arguments:
    /// - `info`: Reference to `CanvasCredentials` containing API URL and token.
    ///
    /// Returns:
    /// - `CanvasResultCourses`: Enum indicating success with course list or an error.
    ///
    /// Example:
    /// ```
    /// let canvas_info = CanvasCredentials { /* initialization */ };
    /// match Canvas::fetch_courses_with_credentials(&canvas_info) {
    ///     CanvasResultCourses::Ok(courses) => /* handle courses */,
    ///     // Handle errors...
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

    /// Fetches a specific course using provided credentials.
    ///
    /// Retrieves details of a single course based on its ID. Utilizes Canvas credentials for authentication.
    ///
    /// Arguments:
    /// - `info`: Canvas API credentials.
    /// - `course_id`: ID of the course to fetch.
    ///
    /// Returns:
    /// - `CanvasResultSingleCourse`: Enum indicating success with the course or an error.
    ///
    /// Example:
    /// ```
    /// let canvas_info = CanvasCredentials { /* initialization */ };
    /// let course_id = 123;
    /// match Canvas::fetch_single_course_with_credentials(&canvas_info, course_id) {
    ///     CanvasResultSingleCourse::Ok(course) => /* handle course */,
    ///     // Handle errors...
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

    /// Converts a JSON object to a `Course`.
    ///
    /// Parses JSON data from the Canvas API to construct a `Course` object.
    ///
    /// Arguments:
    /// - `canvas_info`: Shared credentials reference.
    /// - `course`: JSON object representing a course.
    ///
    /// Returns:
    /// - `Option<Course>`: A course if successful, or `None` if conversion fails.
    ///
    /// Example:
    /// ```
    /// let canvas_info = Arc::new(CanvasCredentials { /* initialization */ });
    /// let course_json = serde_json::json!({ /* JSON data */ });
    /// let course = Canvas::convert_json_to_course(&canvas_info, &course_json);
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
