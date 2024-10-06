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
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use urlencoding::decode;

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
        // let client = &Client::new();
        //
        loop {
            let params = vec![
                (
                    "enrollment_role".to_string(),
                    "TeacherEnrollment".to_string(),
                ),
                ("page".to_string(), page.to_string()),
                ("per_page".to_string(), "100".to_string()),
            ];
            match send_http_request(HttpMethod::Get, &url, &info, params) {
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
                students_cache: Mutex::new(Vec::new()),
                assignments_cache: Mutex::new(Vec::new()),
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

    send_http_request(HttpMethod::Put(body), &url, &canvas_info, vec![])
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
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: u64,
    group_submissions: bool,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/assignments/{}/submissions",
        canvas_info.url_canvas, course_id, assignment_id
    );

    let mut all_submissions = Vec::new();
    let mut page = 1;
    loop {
        let mut params = vec![("page", page.to_string()), ("per_page", "100".to_string())];

        if group_submissions {
            params.push(("grouped", "true".to_string()));
        }

        // Convertendo (&str, String) para (String, String)
        let mut converted_params: Vec<(String, String)> = params
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();

        converted_params.push(("include[]".to_string(), "submission_comments".to_string()));

        match send_http_request(
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

/// Função para buscar as submissões de um estudante para várias tarefas e carregar os file_ids.
pub fn fetch_submissions_for_assignments<F>(
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
        let url = format!(
            "{}/courses/{}/assignments/{}/submissions/{}",
            canvas_info.url_canvas, course_id, assignment_id, user_id
        );

        let params = Vec::new();

        interaction();

        for attempt in 0..SYNC_ATTEMPT {
            let response = send_http_request(HttpMethod::Get, &url, canvas_info, params.clone());

            match response {
                Ok(response) => {
                    if response.status().is_success() {
                        // Deserializar o JSON da resposta uma vez
                        let response_json: Value = response.json()?; // Armazenando o JSON da resposta

                        // Deserializar a submissão do JSON
                        let mut submission: Submission =
                            serde_json::from_value(response_json.clone())?; // Usando clone para reutilizar o JSON

                        // Extrair os file_ids dos anexos (se houver)
                        let file_ids =
                            if let Some(attachments) = response_json["attachments"].as_array() {
                                attachments
                                    .iter()
                                    .filter_map(|file| file["id"].as_u64()) // Extrai os file_ids
                                    .collect()
                            } else {
                                Vec::new() // Caso não haja arquivos, retorna um vetor vazio
                            };

                        // Atribuir os file_ids extraídos à submissão
                        submission.file_ids = file_ids;

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
                        sleep(Duration::from_millis(100));
                    }
                }
            }
        }
    }

    Ok(submissions)
}

pub fn fetch_students(course_info: &CourseInfo) -> Result<Vec<Student>, Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/users",
        course_info.canvas_info.url_canvas, course_info.id
    );

    /// Converts a JSON object to a `Student` structure.
    ///
    /// Parses a JSON representation of a student from the Canvas API into a `Student` object.
    /// Extracts student ID, name, and email and associates it with course information.
    pub fn convert_json_to_student(
        course_info: CourseInfo,
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
                course_info: Arc::new(course_info),
            }),
        })
    }

    let mut all_students = Vec::new();
    let mut page = 1;

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
            HttpMethod::Get, // Supondo que HttpMethod::Get é um enum definido em algum lugar
            &url,
            &course_info.canvas_info,
            converted_params, // Passando o Vec<(String, String)> diretamente
        ) {
            Ok(response) => {
                if response.status().is_success() {
                    let students_page: Vec<serde_json::Value> = response.json()?;
                    if students_page.is_empty() {
                        break; // Sai do loop se não há mais estudantes
                    }
                    all_students.extend(students_page.into_iter().filter_map(|student| {
                        convert_json_to_student(course_info.clone(), &student)
                    }));
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

pub fn convert_json_to_assignment(
    course_info: &Arc<CourseInfo>,
    assignment: &serde_json::Value,
) -> Option<Assignment> {
    let id = assignment["id"].as_u64()?;
    let name = assignment["name"].as_str().map(String::from)?;
    let description = assignment["description"].as_str().map(String::from);

    // Verifica se existe uma rubrica associada e extrai o ID, se disponível
    let rubric_id = assignment["rubric_settings"]["id"].as_u64();

    // Verifica se existe a data de vencimento (due_at) e a parseia se disponível
    let due_at = assignment["due_at"]
        .as_str()
        .map(|due_str| {
            DateTime::parse_from_rfc3339(due_str)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        })
        .flatten(); // Transforma o Result em Option e remove erros de parsing

    // Verifica se o assignment está configurado para submissões em grupo e extrai o group_category_id
    let group_category_id = assignment["group_category_id"].as_u64();

    Some(Assignment {
        info: Arc::new(AssignmentInfo {
            id,
            name,
            description,
            rubric_id, // Armazena o ID da rubrica
            due_at,    // Adiciona o campo due_at (opcional)
            group_category_id,
            course_info: Arc::clone(course_info), // Mantém a referência ao CourseInfo
        }),
    })
}

pub fn fetch_assignments(course: &Course) -> Result<Vec<Assignment>, Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/assignments",
        course.info.canvas_info.url_canvas, course.info.id
    );

    let mut all_assignments = Vec::new();
    let mut page = 1;
    loop {
        let params = vec![("page", page.to_string()), ("per_page", "100".to_string())];

        let converted_params: Vec<(String, String)> = params
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();

        match send_http_request(
            HttpMethod::Get,
            &url,
            &course.info.canvas_info,
            converted_params,
        ) {
            Ok(response) => {
                if response.status().is_success() {
                    let assignments_page: Vec<serde_json::Value> = response.json()?;
                    if assignments_page.is_empty() {
                        break;
                    }
                    all_assignments.extend(assignments_page.into_iter().filter_map(|assignment| {
                        convert_json_to_assignment(&course.info, &assignment)
                    }));
                    page += 1;
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
                        return Err(format!(
                            "Error in upload_binary_file after {} attempts: {}",
                            attempts, e
                        )
                        .into());
                    }
                    sleep(std::time::Duration::from_secs(1)); // Espera 1 segundo antes de tentar novamente
                }
            }
        }
    } else {
        None
    };

    add_comment(
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
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_name: &str,
) -> Result<(), Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/assignments",
        canvas_info.url_canvas, course_id
    );

    let body = json!({
        "assignment": {
            "name": assignment_name,
            "points_possible": 10.0,
            "grading_type": "points",
            "submission_types": ["online_upload"],
            "published": true,
        }
    });

    match send_http_request(HttpMethod::Post(body), &url, canvas_info, vec![]) {
        Ok(response) => {
            if response.status().is_success() {
                println!("Atividade '{}' criada com sucesso!", assignment_name);
                Ok(())
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Falha ao criar atividade com status: {}", response.status()),
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
    canvas_info: &CanvasCredentials,
    course_id: u64,
    title: &str,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    let url = format!(
        "{}/courses/{}/discussion_topics",
        canvas_info.url_canvas, course_id
    );

    let body = json!({
        "title": title,
        "message": message,
        "is_announcement": true
    });

    match send_http_request(HttpMethod::Post(body), &url, canvas_info, vec![]) {
        Ok(response) => {
            if response.status().is_success() {
                Ok(())
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to create announcement with status: {}",
                        response.status()
                    ),
                )))
            }
        }
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create announcement with error: {}", e),
        ))),
    }
}

use crate::rubric_submission::CanvasRubricSubmission;
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::Write;
use std::time::Duration;

/// Downloads a file from the Canvas LMS.
///
/// This function sends an HTTP request to retrieve a file
/// using the Canvas API. It saves the downloaded file locally at the specified path.
///
/// # Arguments
/// - `client`: The reqwest client for making the HTTP request.
/// - `canvas_info`: The CanvasCredentials containing API authentication details.
/// - `file_id`: The ID of the file to be downloaded.
/// - `output_directory`: The path where the file will be saved locally.
///
/// # Returns
/// - `Result<(), Box<dyn Error>>`: Returns Ok on success or an Error on failure.
pub fn download_file(
    client: &Client,
    canvas_info: &CanvasCredentials,
    file_id: u64,
    output_directory: &str, // Directory where the file will be saved
) -> Result<String, Box<dyn std::error::Error>> {
    // Constructing the URL to get the file metadata
    let metadata_url = format!("{}/files/{}", canvas_info.url_canvas, file_id);

    // First, make the request to get the file metadata
    let response = send_http_request(
        HttpMethod::Get, // GET method to retrieve metadata
        &metadata_url,
        canvas_info,
        Vec::new(), // No additional parameters
    )?;

    if response.status().is_success() {
        // Parsing the file metadata
        let metadata: Value = response.json()?;

        // Extracting the original file name and the download URL
        if let (Some(file_name_encoded), Some(download_url)) =
            (metadata["filename"].as_str(), metadata["url"].as_str())
        {
            // Decode the file name (removes encoded characters)
            let file_name_decoded = decode(file_name_encoded)?.into_owned();
            let file_name = file_name_decoded.replace("+", " "); // Replaces '+' with spaces

            // Construct the full path where the file will be saved
            let output_path = Path::new(output_directory).join(&file_name);

            // Now make the request to download the actual file using the download URL
            let file_response = client.get(download_url).send()?;

            if file_response.status().is_success() {
                // Save the file content to the specified output path
                let mut file = File::create(&output_path)?;
                let content = file_response.bytes()?;
                file.write_all(&content)?;

                // println!("File '{}' successfully downloaded to: {}", file_name, output_path.display());
                Ok(output_path.to_string_lossy().into_owned()) // Return the path to the saved file
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to download the file. Status: {}",
                        file_response.status()
                    ),
                )))
            }
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "The download URL or file name was not found in the metadata.".to_string(),
            )))
        }
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Failed to retrieve file metadata. Status: {}",
                response.status()
            ),
        )))
    }
}

pub fn download_rubric(
    canvas_info: &CanvasCredentials,
    course_id: u64,
    rubric_id: u64,
) -> Result<Value, Box<dyn Error>> {
    // URL para obter os detalhes da rubrica
    let url = format!(
        "{}/courses/{}/rubrics/{}",
        canvas_info.url_canvas, course_id, rubric_id
    );

    // Parâmetros adicionais, se necessário (neste caso, nenhum parâmetro extra)
    let params = Vec::new();

    // Realiza a requisição HTTP
    match send_http_request(HttpMethod::Get, &url, canvas_info, params) {
        Ok(response) => {
            if response.status().is_success() {
                // Parseia o JSON retornado pela resposta
                let rubric_details: Value = response.json()?;
                Ok(rubric_details) // Retorna o JSON com os detalhes da rubrica
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to download rubric with status: {}",
                        response.status()
                    ),
                )))
            }
        }
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to download rubric with error: {}", e),
        ))),
    }
}

/// Função para criar uma rubrica no Canvas LMS.
pub fn create_rubric(
    canvas_info: &CanvasCredentials,
    course_id: u64,
    rubric: &CanvasRubricSubmission, // Using CanvasRubricSubmission instead of Rubric
) -> Result<(), Box<dyn Error>> {
    // URL for the API to create the rubric
    let url = format!("{}/courses/{}/rubrics", canvas_info.url_canvas, course_id);

    // Serializing the CanvasRubricSubmission structure to JSON, with numerical string keys for criteria and ratings
    let rubric_data = json!({
        "rubric": {
            "title": rubric.rubric.title,
            "criteria": rubric.rubric.criteria.iter().map(|(key, criterion)| {
                (
                    key.clone(), // Dereferencing the key (from &String to String)
                    json!({
                        "description": criterion.description,
                        "criterion_use_range": criterion.criterion_use_range,
                        "ratings": criterion.ratings.iter().map(|(rating_key, rating)| {
                            (
                                rating_key.clone(), // Dereferencing the rating key (from &String to String)
                                json!({
                                    "description": rating.description,
                                    "points": rating.points
                                })
                            )
                        }).collect::<serde_json::Map<String, serde_json::Value>>() // Collecting into Map<String, Value>
                    })
                )
            }).collect::<serde_json::Map<String, serde_json::Value>>() // Collecting into Map<String, Value>
        },
        "rubric_association": {
            "association_type": rubric.rubric_association.association_type,
            "association_id": rubric.rubric_association.association_id,
            "use_for_grading": rubric.rubric_association.use_for_grading
        }
    });

    // Sending the POST request using send_http_request
    let response = send_http_request(
        HttpMethod::Post(rubric_data), // Sending the JSON body with the POST request
        &url,
        canvas_info,
        vec![], // No additional parameters
    )?;

    // Checking if the response was successful
    if response.status().is_success() {
        // let resp_json: serde_json::Value = response.json()?;
        // println!("Rubric created successfully: {:?}", resp_json);
        Ok(())
    } else {
        // let error_text = response.text()?;
        // println!("Failed to create rubric: {}", error_text);
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Error creating rubric",
        )))
    }
}

/// Função para apagar um comentário de uma submissão no Canvas.
///
/// Esta função envia uma requisição DELETE para o Canvas API para remover um comentário
/// de uma submissão de um estudante em uma atividade.
///
/// # Argumentos
///
/// - `client`: Cliente HTTP para realizar as requisições.
/// - `canvas_info`: Credenciais do Canvas, incluindo o token de acesso.
/// - `course_id`: ID do curso onde está a submissão.
/// - `assignment_id`: ID da atividade onde a submissão foi realizada.
/// - `user_id`: ID do estudante que realizou a submissão.
/// - `comment_id`: ID do comentário que será apagado.
///
/// # Retorno
///
/// Retorna um `Result<(), Box<dyn Error>>` que indica sucesso ou falha na operação.
pub fn delete_comment(
    canvas_info: &CanvasCredentials,
    course_id: u64,
    assignment_id: u64,
    user_id: u64,
    comment_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Montar a URL para apagar o comentário
    let url = format!(
        "{}/courses/{}/assignments/{}/submissions/{}/comments/{}",
        canvas_info.url_canvas, course_id, assignment_id, user_id, comment_id
    );

    // Chamar send_http_request usando o método DELETE
    let response = send_http_request(
        HttpMethod::Delete, // Usando o novo método DELETE
        &url,
        canvas_info,
        vec![], // Nenhum parâmetro adicional necessário
    )?;

    // Verifica se a requisição foi bem-sucedida
    if response.status().is_success() {
        Ok(())
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Falha ao apagar comentário: HTTP {}", response.status()),
        )))
    }
}

fn fetch_groups_for_category(
    group_category_id: u64,
    canvas_info: &CanvasCredentials,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/group_categories/{}/groups",
        canvas_info.url_canvas, group_category_id
    );
    let response = send_http_request(HttpMethod::Get, &url, canvas_info, vec![])?;
    let groups: Vec<serde_json::Value> = response.json()?;
    Ok(groups)
}

pub fn fetch_groups_for_assignment(
    assignment_info: &AssignmentInfo,
    canvas_info: &CanvasCredentials,
) -> Result<HashMap<u64, Vec<u64>>, Box<dyn std::error::Error>> {
    let mut group_student_map = HashMap::new();

    // Verificar se o assignment possui um `group_category_id`
    if let Some(group_category_id) = assignment_info.group_category_id {
        // Obter os grupos da categoria de grupo
        let groups = fetch_groups_for_category(group_category_id, canvas_info)?;

        // Itera sobre os grupos e busca os estudantes de cada grupo
        for group in groups {
            if let Some(group_id) = group["id"].as_u64() {
                let group_url = format!("{}/groups/{}/users", canvas_info.url_canvas, group_id);
                let group_response =
                    send_http_request(HttpMethod::Get, &group_url, canvas_info, vec![])?;
                let users: Vec<serde_json::Value> = group_response.json()?;

                // Coleta os IDs dos estudantes para o grupo
                let mut student_ids = Vec::new();
                for user in users {
                    if let Some(student_id) = user["id"].as_u64() {
                        student_ids.push(student_id);
                    }
                }

                // Adiciona o grupo e os estudantes ao mapa
                group_student_map.insert(group_id, student_ids);
            }
        }
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Assignment is not configured for group submissions",
        )));
    }

    Ok(group_student_map)
}
