use crate::connection::{send_http_request, HttpMethod, SYNC_ATTEMPT};
use crate::{
    course, Assignment, AssignmentInfo, CanvasCredentials, Course, CourseInfo, Student,
    StudentInfo, Submission,
};
use course::parse_course_name;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::thread::sleep;
use serde_json::json;

/// Enum to represent the result of fetching multiple courses.
///
/// This enum provides a structured way to handle the outcomes of attempting to fetch a list of courses
/// from the Canvas LMS, distinguishing between successful retrieval, connection errors, and credential errors.
pub enum CanvasResultCourses {
    Ok(Vec<Course>),        // Success case with a vector of Course objects.
    ErrConnection(String),  // Connection error with a descriptive message.
    ErrCredentials(String), // Credential error with a descriptive message.
}

/// Enum to represent the result of fetching a single course.
///
/// Similar to `CanvasResultCourses`, but tailored for scenarios where only a single course is being fetched.
/// Distinguishes between success, connection errors, and credential errors.
pub enum CanvasResultSingleCourse {
    Ok(Course),             // Success case with a single Course object.
    ErrConnection(String),  // Connection error with a descriptive message.
    ErrCredentials(String), // Credential error with a descriptive message.
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
        let client = &Client::new();

        loop {
            let params = vec![
                (
                    "enrollment_role".to_string(),
                    "TeacherEnrollment".to_string(),
                ),
                ("page".to_string(), page.to_string()),
                ("per_page".to_string(), "100".to_string()),
            ];
            match send_http_request(&client, HttpMethod::Get, &url, &info, params) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text() {
                            Ok(text) => {
                                // println!("Response Text: {}", text);

                                // Se precisar processar como JSON, converta novamente
                                match serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                                    Ok(courses) => {
                                        if courses.is_empty() {
                                            break; // Sai do loop se nenhum curso for retornado
                                        }
                                        all_courses.extend(courses.iter().filter_map(|course| {
                                            Canvas::convert_json_to_course(&canvas_info_arc, course)
                                        }));
                                        page += 1; // Incrementa o número da página
                                    }
                                    Err(e) => {
                                        // error!("Failed to parse courses JSON with error: {}", e);
                                        return CanvasResultCourses::ErrCredentials(format!(
                                            "Failed to parse courses JSON with error: {}",
                                            e
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                // error!("Failed to read response text with error: {}", e);
                                return CanvasResultCourses::ErrCredentials(format!(
                                    "Failed to read response text with error: {}",
                                    e
                                ));
                            }
                        }
                    } else {
                        // error!("Failed to fetch courses with status: {}", response.status());
                        return CanvasResultCourses::ErrCredentials(format!(
                            "Failed to fetch courses with status: {}",
                            response.status()
                        ));
                    }
                }
                Err(e) => {
                    // error!("Failed to fetch courses with error: {}", e);
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
            &Client::new(),
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
                        return CanvasResultSingleCourse::ErrConnection(
                            "Failed to parse course data".to_string(),
                        );
                    }
                } else {
                    CanvasResultSingleCourse::ErrConnection(format!(
                        "Failed to fetch course: HTTP Status {}",
                        response.status()
                    ))
                }
            }
            Err(e) => {
                CanvasResultSingleCourse::ErrConnection(format!("HTTP request failed: {}", e))
            }
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
                name: name.clone(),
                course_code: course_code.clone(),
                canvas_info: Arc::clone(canvas_info),
                abbreviated_name: parse_course_name(name.as_str(), course_code.as_str()), // Parse the course name
            }),
        })
    }

    pub fn choose_course() -> Option<Course> {
        let mut menu_str = Vec::new();
        let mut menu_course = Vec::new();

        let credentials = CanvasCredentials::credentials();
        println!("Fetching courses...");
        match Canvas::fetch_courses_with_credentials(&credentials) {
            CanvasResultCourses::Ok(courses) => {
                for course in courses {
                    if let Some(course_details_name) = parse_course_name(
                        course.info.name.as_str(),
                        course.info.course_code.as_str(),
                    ) {
                        menu_str.push(course_details_name.abbreviated_name);
                        menu_course.push(course);
                    }
                }
            }
            CanvasResultCourses::ErrConnection(msg) => {
                eprintln!("Connection error: {}", msg);
                std::process::exit(1);
            }
            CanvasResultCourses::ErrCredentials(msg) => {
                eprintln!("Credential error: {}", msg);
                std::process::exit(1);
            }
        }

        // Add EXIT at the end of the list
        menu_str.push("EXIT".to_string());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose a course")
            .items(&menu_str)
            .default(0)
            .interact()
            .unwrap();

        if selection == menu_str.len() - 1 {
            return None;
        }

        Some(menu_course[selection].clone())
    }
}

/// Returns the current year and semester as a tuple of strings.
///
/// The year is represented as a four-digit number (e.g., "2023"). The semester
/// is determined based on the current month and day: "1" for dates on or before
/// July 15 (first semester), and "2" for dates after July 15 (second semester).
///
/// # Examples
///
/// ```
/// let (year, semester) = get_current_year_and_semester();
/// println!("Year: {}, Semester: {}", year, semester);
/// ```
///
/// # Errors
///
/// This function does not return any errors. It will always provide the current year and
/// the calculated semester based on the current date.
pub fn get_current_year_and_semester() -> (String, String) {
    use chrono::{Datelike, Utc};

    let current_date = Utc::now();
    let year = current_date.year().to_string();
    let semester =
        if current_date.month() < 7 || (current_date.month() == 7 && current_date.day() <= 15) {
            "1".to_string()
        } else {
            "2".to_string()
        };

    (year, semester)
}

/// Adds a comment to a student's assignment submission.
///
/// This function sends an HTTP PUT request to add a comment to a specific
/// assignment submission by a student. Optionally, it can include file IDs
/// associated with the comment.
///
/// Arguments:
/// - `client`: HTTP client for executing requests.
/// - `assignment_id`: ID of the assignment.
/// - `user_id`: ID of the user (student).
/// - `comment_text`: Text content of the comment.
/// - `file_ids`: Optional vector of file IDs to be attached to the comment.
///
/// Returns:
/// - `Result<(), Box<dyn Error>>`: Success or an error detailing any issues encountered.
///
/// Example:
/// ```
/// let client = Client::new();
/// let course = Course { /* ... */ };
/// match course.add_comment(&client, "assignment_id", "user_id", "Great work!", None) {
///     Ok(_) => /* handle success */,
///     Err(e) => /* handle error */,
/// }
/// ```
fn add_comment(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: &str,
    user_id: &str,
    comment_text: &str,
    file_ids: Option<Vec<i64>>,
) -> Result<(), Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/assignments/{}/submissions/{}",
        canvas_info.url_canvas, course_id, assignment_id, user_id
    );

    let mut body = serde_json::json!({
        "comment": {
            "text_comment": comment_text
        }
    });

    if let Some(file_ids) = file_ids {
        body["comment"]["file_ids"] = serde_json::json!(file_ids);
    }

    send_http_request(client, HttpMethod::Put(body), &url, &canvas_info, vec![])
        .map_err(|e| format!("Failed to add comment: {}", e))?;
    Ok(())
}

/// Requests an upload token from the Canvas LMS.
///
/// This function sends an HTTP POST request to the Canvas LMS to request an upload token
/// for uploading a file. It requires details about the assignment, user, file name, and file size.
///
/// Arguments:
/// - `client`: HTTP client for executing requests.
/// - `assignment_id`: ID of the assignment.
/// - `user_id`: ID of the user (student).
/// - `file_name`: Name of the file to be uploaded.
/// - `file_size`: Size of the file to be uploaded.
///
/// Returns:
/// - `Result<(String, HashMap<String, String>), Box<dyn Error>>`: Tuple containing the upload URL
///   and a map of upload parameters if successful, or an error detailing any issues encountered.
///
/// Example:
/// ```
/// let client = Client::new();
/// let course = Course { /* ... */ };
/// match course.request_upload_token(&client, "assignment_id", "user_id", "test.pdf", 12345) {
///     Ok((upload_url, upload_params)) => /* handle success */,
///     Err(e) => /* handle error */,
/// }
/// ```
pub fn request_upload_token(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: &str,
    user_id: &str,
    file_name: &str,
    file_size: u64,
) -> Result<(String, HashMap<String, String>), Box<dyn Error>> {
    // Construindo a URL de solicitação
    let url = format!(
        "{}/courses/{}/assignments/{}/submissions/{}/comments/files",
        canvas_info.url_canvas, course_id, assignment_id, user_id
    );

    // Construindo o corpo da requisição
    let body = serde_json::json!({
        "name": file_name,
        "size": file_size
    });

    // Enviando a solicitação HTTP
    match send_http_request(
        client,
        HttpMethod::Post(body), // Usar a variante HttpMethod::Post com corpo JSON
        &url,
        &canvas_info,
        vec![], // POST request não necessita de params
    ) {
        Ok(response) => {
            // Verificando se a resposta foi bem-sucedida
            if response.status().is_success() {
                // Parseando a resposta JSON
                let json_response: serde_json::Value = response.json()?;
                let upload_url = json_response["upload_url"]
                    .as_str()
                    .ok_or("Missing upload_url")?
                    .to_string();
                let upload_params = json_response["upload_params"]
                    .as_object()
                    .ok_or("Missing upload_params")?;

                let mut params = HashMap::new();
                for (key, value) in upload_params {
                    let value_str = value.as_str().ok_or("Invalid param value")?;
                    params.insert(key.clone(), value_str.to_string());
                }

                Ok((upload_url, params))
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to request upload token with status: {}",
                        response.status()
                    ),
                )))
            }
        }
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to request upload token with error: {}", e),
        ))),
    }
}

/// Uploads a file to the Canvas LMS.
///
/// This function handles the file upload process by first requesting an upload token
/// and then using that token to upload the file. It reads the file content and sends it
/// as a multipart/form-data request to the provided upload URL.
///
/// Arguments:
/// - `client`: HTTP client for executing requests.
/// - `assignment_id`: ID of the assignment.
/// - `user_id`: ID of the user (student).
/// - `file_path`: Path to the file to be uploaded.
///
/// Returns:
/// - `Result<i64, Box<dyn Error>>`: File ID if the upload is successful, or an error detailing
///   any issues encountered.
///
/// Example:
/// ```
/// let client = Client::new();
/// let course = Course { /* ... */ };
/// match course.upload_file(&client, "assignment_id", "user_id", "path/to/file.pdf") {
///     Ok(file_id) => /* handle success */,
///     Err(e) => /* handle error */,
/// }
/// ```
fn upload_file(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: &str,
    user_id: &str,
    file_path: &str,
) -> Result<i64, Box<dyn Error>> {
    use std::fs::File;
    use std::io::Read;

    let file_name = std::path::Path::new(file_path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or("Invalid file name")?;

    let file_size = std::fs::metadata(file_path)?.len();

    match request_upload_token(
        client,
        canvas_info,
        course_id,
        assignment_id,
        user_id,
        file_name,
        file_size,
    ) {
        Ok((upload_url, upload_params)) => {
            // println!("Received upload token: {}", upload_url);
            // println!("Received upload params: {:?}", upload_params);

            let mut file = File::open(file_path)?;
            let mut file_content = Vec::new();
            file.read_to_end(&mut file_content)?;

            let mut form = Form::new();
            for (key, value) in upload_params {
                form = form.text(key, value);
            }
            form = form.file("file", file_path)?;

            let response = client
                .post(&upload_url)
                .multipart(form)
                .send()
                .map_err(|e| format!("Failed to upload file: {}", e))?;

            let json: Value = response
                .json()
                .map_err(|e| format!("Failed to parse upload file response: {}", e))?;

            // println!("Upload response JSON: {:?}", json);

            let file_id = json["id"]
                .as_i64()
                .ok_or("Missing id in upload file response")?;

            Ok(file_id)
        }
        Err(e) => {
            return Err(format!("Failed to request upload token: {}", e).into());
        }
    }
}

/// Adds a file comment to a student's assignment submission.
///
/// This function first uploads a file to the Canvas LMS and then attaches it as a comment
/// to a specific assignment submission by a student. It also adds text content to the comment.
///
/// Arguments:
/// - `client`: HTTP client for executing requests.
/// - `assignment_id`: ID of the assignment.
/// - `student_id`: ID of the student.
/// - `file_path`: Optional path to the file to be uploaded.
/// - `comment_text`: Text content of the comment.
///
/// Returns:
/// - `Result<(), Box<dyn Error>>`: Success or an error detailing any issues encountered.
///
/// Example:
/// ```
/// let client = Client::new();
/// let course = Course { /* ... */ };
/// match course.add_file_comment(&client, assignment_id, student_id, Some("path/to/file.pdf"), "Great work!") {
///     Ok(_) => /* handle success */,
///     Err(e) => /* handle error */,
/// }
/// ```
pub fn comment_with_file(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: u64,
    student_id: u64,
    file_path: Option<&str>,
    comment_text: &str,
) -> Result<(), Box<dyn Error>> {
    // println!("Course ID: {}", self.info.id);
    // println!("Assignment ID: {}", assignment_id);
    // println!("Student ID: {}", student_id);

    let user_id = student_id.to_string();
    let assignment_id_str = assignment_id.to_string();

    let file_ids = if let Some(path) = file_path {
        let file_id = upload_file(
            client,
            canvas_info,
            course_id,
            &assignment_id_str,
            &user_id,
            path,
        )
        .map_err(|e| format!("Error in upload_file: {}", e))?;
        Some(vec![file_id])
    } else {
        None
    };

    add_comment(
        client,
        canvas_info,
        course_id,
        &assignment_id_str,
        &user_id,
        comment_text,
        file_ids,
    )
    .map_err(|e| format!("Error in add_comment: {}", e))?;

    Ok(())
}

/// Retrieves all submissions for a specific assignment for all students in a course.
///
/// This function sends an HTTP GET request to the Canvas LMS API to retrieve all submissions
/// for a given assignment in a specified course.
///
/// Arguments:
/// - `client`: HTTP client for executing requests.
/// - `canvas_info`: Reference to Canvas credentials and configuration.
/// - `course_id`: ID of the course.
/// - `assignment_id`: ID of the assignment.
///
/// Returns:
/// - `Result<serde_json::Value, Box<dyn Error>>`: JSON response containing the submissions
///   or an error detailing any issues encountered.
///
/// Example:
/// ```
/// let client = Client::new();
/// let canvas_info = CanvasCredentials { /* ... */ };
/// match get_all_submissions(&client, &canvas_info, 32451, "174964") {
///     Ok(submissions) => /* handle success */,
///     Err(e) => /* handle error */,
/// }
/// ```
pub fn get_all_submissions(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: u64,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/assignments/{}/submissions",
        canvas_info.url_canvas, course_id, assignment_id
    );

    let mut all_submissions = Vec::new();
    let mut page = 1;
    loop {
        let params = vec![("page", page.to_string()), ("per_page", "100".to_string())];

        // Convertendo (&str, String) para (String, String)
        let converted_params: Vec<(String, String)> = params
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();

        match send_http_request(
            client,
            HttpMethod::Get,
            &url,
            canvas_info,
            converted_params, // Passando o Vec<(String, String)> diretamente
        ) {
            Ok(response) => {
                if response.status().is_success() {
                    let submissions_page: Vec<Value> = response.json()?;
                    if submissions_page.is_empty() {
                        break; // Sai do loop se não há mais submissões
                    }
                    all_submissions.extend(submissions_page);
                    page += 1; // Incrementa o número da página para a próxima iteração
                } else {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Failed to fetch submissions with status: {}",
                            response.status()
                        ),
                    )));
                }
            }
            Err(e) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to fetch submissions with error: {}", e),
                )));
            }
        }
    }
    Ok(all_submissions)
}

pub fn fetch_submissions_for_assignments<F>(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    user_id: u64,
    assignment_ids: &[u64],
    interaction: F,
) -> Result<Vec<Submission>, Box<dyn std::error::Error>>
where
    F: Fn(),
{
    let mut submissions = Vec::new();

    for &assignment_id in assignment_ids {
        // update_carrossel();
        let url = format!(
            "{}/courses/{}/assignments/{}/submissions/{}",
            canvas_info.url_canvas, course_id, assignment_id, user_id
        );

        // Não são necessários parâmetros adicionais para esta chamada de API específica
        let params = Vec::new(); // Sem parâmetros adicionais para a requisição GET

        interaction();
        // Try to send the HTTP request SYNC_ATTEMPT times
        for attempt in 0..SYNC_ATTEMPT {
            let response = send_http_request(
                client,
                HttpMethod::Get, // Método GET
                &url,            // URL da API
                &canvas_info,    // Token de acesso
                params.clone(),  // Parâmetros da requisição
            );
            match response {
                Ok(response) => {
                    if response.status().is_success() {
                        let submission: Submission = response.json()?; // Deserializar a resposta JSON para um objeto Submission
                        submissions.push(submission);
                    } else {
                        let error_message = response.text()?;
                        return Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!(
                                "Failed to fetch submissions with error: {} (a)",
                                error_message
                            ),
                        )));
                    }
                    break;
                }
                Err(e) => {
                    if attempt == SYNC_ATTEMPT - 1 {
                        return Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to fetch submissions with error: {} (b)", e),
                        )));
                    } else {
                        sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        }
    }
    Ok(submissions)
}

pub fn fetch_students(course: &Course) -> Result<Vec<Student>, Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/users",
        course.info.canvas_info.url_canvas, course.info.id
    );

    /// Converts a JSON object to a `Student` structure.
    ///
    /// Parses a JSON representation of a student from the Canvas API into a `Student` object.
    /// Extracts student ID, name, and email and associates it with course information.
    pub fn convert_json_to_student(
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

    let mut all_students = Vec::new();
    let mut page = 1;
    let client = &Client::new();

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
        match send_http_request(
            client,
            HttpMethod::Get, // Supondo que HttpMethod::Get é um enum definido em algum lugar
            &url,
            &course.info.canvas_info,
            converted_params, // Passando o Vec<(String, String)> diretamente
        ) {
            Ok(response) => {
                if response.status().is_success() {
                    let students_page: Vec<serde_json::Value> = response.json()?;
                    if students_page.is_empty() {
                        break; // Sai do loop se não há mais estudantes
                    }
                    all_students.extend(
                        students_page
                            .into_iter()
                            .filter_map(|student| convert_json_to_student(&course.info, &student)),
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
            Err(e) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to fetch students with error: {}", e),
                )));
            }
        }
    }
    Ok(all_students)
}

pub fn fetch_assignments(course: &Course) -> Result<Vec<Assignment>, Box<dyn Error>> {
    /// Converts a JSON object into an `Assignment` structure.
    ///
    /// Transforms a JSON representation of an assignment into an `Assignment` object. Retrieves key
    /// details such as ID, name, and description, linking them with the course information.
    pub fn convert_json_to_assignment(
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

    let url = format!(
        "{}/courses/{}/assignments",
        course.info.canvas_info.url_canvas, course.info.id
    );

    let mut all_assignments = Vec::new();
    let mut page = 1;
    let client = &reqwest::blocking::Client::new();
    loop {
        // Construindo os parâmetros da requisição
        let params = vec![("page", page.to_string()), ("per_page", "100".to_string())];

        // Convertendo (&str, String) para (String, String)
        let converted_params: Vec<(String, String)> = params
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();

        match send_http_request(
            client,
            HttpMethod::Get,
            &url,
            &course.info.canvas_info,
            converted_params, // Passando o Vec<(String, String)> diretamente
        ) {
            Ok(response) => {
                if response.status().is_success() {
                    let assignments_page: Vec<serde_json::Value> = response.json()?;
                    if assignments_page.is_empty() {
                        break; // Sai do loop se não há mais cursos
                    }
                    all_assignments.extend(assignments_page.into_iter().filter_map(|assignment| {
                        convert_json_to_assignment(&course.info, &assignment)
                    }));
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
            Err(e) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to fetch assignments with error: {}", e),
                )));
            }
        }
    }
    Ok(all_assignments)
}

pub fn update_assignment_score(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: u64,
    student_id: u64,
    new_score: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/assignments/{}/submissions/{}",
        canvas_info.url_canvas, course_id, assignment_id, student_id,
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

    // Try to send the HTTP request SYNC_ATTEMPT times
    let mut attempt = SYNC_ATTEMPT;
    loop {
        match send_http_request(
            client,
            HttpMethod::Put(body.clone()), // Use HttpMethod::Put enum variant
            &url,
            &canvas_info,
            Vec::new(), // PUT request does not need params
        ) {
            Ok(response) => match response.status().is_success() {
                true => return Ok(()),
                false => {
                    if attempt == 0 {
                        return Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to update score with status: {}", response.status()),
                        )));
                    }
                }
            },
            Err(e) => {
                if attempt == 0 {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to update score with error: {}", e),
                    )));
                }
            }
        };
        attempt -= 1;
        sleep(std::time::Duration::from_millis(100));
    }
}

pub fn comment_with_binary_file(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: u64,
    student_id: u64,
    file_name: Option<&str>,
    file_content: Option<&Vec<u8>>,
    comment_text: &str,
) -> Result<(), Box<dyn Error>> {
    let user_id = student_id.to_string();
    let assignment_id_str = assignment_id.to_string();

    let file_ids = if let (Some(name), Some(content)) = (file_name, file_content) {
        let mut attempts = 0;
        let max_attempts = 3;
        loop {
            match upload_binary_file(
                client,
                canvas_info,
                course_id,
                &assignment_id_str,
                &user_id,
                name,
                content,
            ) {
                Ok(file_id) => break Some(vec![file_id]),
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(format!("Error in upload_binary_file after {} attempts: {}", attempts, e).into());
                    }
                    sleep(std::time::Duration::from_secs(1)); // Espera 1 segundo antes de tentar novamente
                }
            }
        }
    } else {
        None
    };

    add_comment(
        client,
        canvas_info,
        course_id,
        &assignment_id_str,
        &user_id,
        comment_text,
        file_ids,
    )
        .map_err(|e| format!("Error in add_comment: {}", e))?;

    Ok(())
}


fn upload_binary_file(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: &str,
    user_id: &str,
    file_name: &str,
    file_content: &Vec<u8>,
) -> Result<i64, Box<dyn Error>> {
    let file_size = file_content.len() as u64;

    match request_upload_token(
        client,
        canvas_info,
        course_id,
        assignment_id,
        user_id,
        file_name,
        file_size,
    ) {
        Ok((upload_url, upload_params)) => {
            let mut form = Form::new();
            for (key, value) in upload_params {
                form = form.text(key, value);
            }
            form = form.part(
                "file",
                Part::bytes(file_content.clone()).file_name(file_name.to_string()),
            );

            let response = client
                .post(&upload_url)
                .multipart(form)
                .send()
                .map_err(|e| format!("Failed to upload file: {}", e))?;

            let json: Value = response
                .json()
                .map_err(|e| format!("Failed to parse upload file response: {}", e))?;

            let file_id = json["id"]
                .as_i64()
                .ok_or("Missing id in upload file response")?;

            Ok(file_id)
        }
        Err(e) => {
            return Err(format!("Failed to request upload token: {}", e).into());
        }
    }
}

/// Cria uma nova atividade (assignment) em um curso no Canvas.
///
/// Esta função envia uma solicitação HTTP POST para a API do Canvas para criar uma nova atividade.
/// Requer o ID do curso, o nome da atividade e as credenciais do Canvas para autenticação.
///
/// Argumentos:
/// - `client`: Cliente HTTP para executar as requisições.
/// - `canvas_info`: Referência para as credenciais do Canvas.
/// - `course_id`: ID do curso onde a atividade será criada.
/// - `assignment_name`: Nome da nova atividade.
///
/// Retorna:
/// - `Result<(), Box<dyn Error>>`: Sucesso ou um erro detalhando quaisquer problemas encontrados.
///
/// Exemplo:
/// ```
/// let client = Client::new();
/// let canvas_info = CanvasCredentials { /* ... */ };
/// match create_assignment(&client, &canvas_info, 12345, "Nova Atividade") {
///     Ok(_) => println!("Atividade criada com sucesso!"),
///     Err(e) => eprintln!("Erro ao criar atividade: {}", e),
/// }
/// ```
pub fn create_assignment(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_name: &str,
) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/courses/{}/assignments", canvas_info.url_canvas, course_id);

    let body = json!({
        "assignment": {
            "name": assignment_name,
            "points_possible": 10.0,
            "grading_type": "points",
            "submission_types": ["online_upload"],
            "published": true,
        }
    });

    match send_http_request(client, HttpMethod::Post(body), &url, canvas_info, vec![]) {
        Ok(response) => {
            if response.status().is_success() {
                println!("Atividade '{}' criada com sucesso!", assignment_name);
                Ok(())
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Falha ao criar atividade com status: {}",
                        response.status()
                    ),
                )))
            }
        }
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Falha ao criar atividade com erro: {}", e),
        ))),
    }
}

pub fn create_announcement(
    client: &Client,
    canvas_info: &CanvasCredentials,
    course_id: u64,
    title: &str,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/courses/{}/discussion_topics", canvas_info.url_canvas, course_id);

    let body = json!({
        "title": title,
        "message": message,
        "is_announcement": true
    });

    match send_http_request(
        client,
        HttpMethod::Post(body),
        &url,
        canvas_info,
        vec![],
    ) {
        Ok(response) => {
            if response.status().is_success() {
                Ok(())
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create announcement with status: {}", response.status()),
                )))
            }
        }
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create announcement with error: {}", e),
        ))),
    }
}
