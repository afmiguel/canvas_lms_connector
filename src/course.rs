// Necessary imports from standard and external crates.
use crate::assignment::Assignment;
use crate::student::Student;
use crate::{canvas, Canvas, CanvasCredentials, CanvasResultSingleCourse};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::process::exit;
use std::sync::Arc;
use std::sync::Mutex;

/// Structure holding detailed information about a Canvas course.
///
/// This structure encapsulates essential details of a course, such as its identifier, name, and code.
/// It plays a critical role in providing context for API interactions within the Canvas LMS, such as fetching
/// course-specific data or submitting changes.
///
/// Fields:
/// - `id`: Unique identifier of the course in the Canvas system.
/// - `name`: Official name of the course.
/// - `course_code`: Short identifier or code for the course.
/// - `canvas_info`: Shared reference to Canvas credentials and API URL, enabling API interactions.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CourseInfo {
    pub id: u64,
    pub name: String,
    pub course_code: String,
    #[serde(skip)]
    pub canvas_info: Arc<CanvasCredentials>,
    #[serde(skip)]
    pub abbreviated_name: Option<CourseNameDetails>,
    #[serde(skip)]
    pub students_cache: Mutex<Vec<Student>>,
    #[serde(skip)]
    pub assignments_cache: Mutex<Vec<Assignment>>,
}

/// High-level representation of a Canvas course.
///
/// This structure is a wrapper around `CourseInfo`, providing a convenient interface to manage and access
/// course-related information in the Canvas LMS. It is used in various operations and API interactions that
/// involve courses.
///
/// Fields:
/// - `info`: Shared reference to `CourseInfo` for accessing detailed course information.
#[derive(Clone)]
pub struct Course {
    pub info: Arc<CourseInfo>,
}

// Implementando Clone manualmente
impl Clone for CourseInfo {
    fn clone(&self) -> Self {
        CourseInfo {
            id: self.id,
            name: self.name.clone(),
            course_code: self.course_code.clone(),
            canvas_info: Arc::clone(&self.canvas_info),
            abbreviated_name: self.abbreviated_name.clone(),
            students_cache: Mutex::new(self.students_cache.lock().unwrap().clone()),
            assignments_cache: Mutex::new(self.assignments_cache.lock().unwrap().clone()),
        }
    }
}

impl CourseInfo {
    /// Retrieves students enrolled in this course.
    ///
    /// Makes a Canvas API call to fetch a list of students enrolled in the course. Utilizes course ID
    /// and API credentials from `CourseInfo` for authentication. Handles API pagination to ensure all
    /// students are retrieved.
    ///
    /// Returns:
    /// - `Result<Vec<Student>, Box<dyn std::error::Error>>`: Success with a list of students or an error
    ///   detailing any issues encountered during the API call.
    ///
    /// Example:
    /// ```
    /// let course_info = CourseInfo { /* ... */ };
    /// match course_info.fetch_students() {
    ///     Ok(students) => /* handle students */,
    ///     Err(e) => /* handle error */,
    /// }
    /// ```
    pub fn fetch_students(&self) -> Result<Vec<Student>, Box<dyn Error>> {
        {
            let students_cache = self.students_cache.lock().unwrap();
            if !students_cache.is_empty() {
                return Ok(students_cache.clone());
            }
        }
        match canvas::fetch_students(self) {
            Ok(students) => {
                let mut students_cache = self.students_cache.lock().unwrap();
                students_cache.extend(students.clone());
                Ok(students_cache.to_vec())
            }
            Err(e) => Err(e),
        }
    }

    pub fn clear_cache(&self) {
        let mut students_cache = self.students_cache.lock().unwrap();
        students_cache.clear();
        let mut assignments_cache = self.assignments_cache.lock().unwrap();
        assignments_cache.clear();
    }
}

/// Implementation of methods for the `Course` struct, targeting course-specific functionalities in Canvas.
///
/// These methods provide capabilities such as retrieving enrolled students, fetching assignments, and
/// updating assignment scores. They are integral for applications that interact with the Canvas API to
/// manage course data.
impl Course {
    /// Retrieves students enrolled in this course.
    ///
    /// Makes a Canvas API call to fetch a list of students enrolled in the course. Utilizes course ID
    /// and API credentials from `CourseInfo` for authentication. Handles API pagination to ensure all
    /// students are retrieved.
    ///
    /// Returns:
    /// - `Result<Vec<Student>, Box<dyn std::error::Error>>`: Success with a list of students or an error
    ///   detailing any issues encountered during the API call.
    ///
    /// Example:
    /// ```
    /// let course = Course { /* ... */ };
    /// match course.fetch_students() {
    ///     Ok(students) => /* handle students */,
    ///     Err(e) => /* handle error */,
    /// }
    /// ```
    pub fn fetch_students(&self) -> Result<Vec<Student>, Box<dyn Error>> {
        self.info.fetch_students()
    }

    pub fn clear_cache(&self) {
        self.info.clear_cache();
    }

    /// Retrieves assignments for this course.
    ///
    /// Queries the Canvas API to fetch all assignments related to the course. Uses course ID and
    /// API credentials for authenticated requests. Manages API pagination to collect all assignments.
    ///
    /// Returns:
    /// - `Result<Vec<Assignment>, Box<dyn std::error::Error>>`: Success with a vector of assignments or
    ///   an error detailing any API call issues.
    ///
    /// Example:
    /// ```
    /// let course = Course { /* ... */ };
    /// match course.fetch_assignments() {
    ///     Ok(assignments) => /* handle assignments */,
    ///     Err(e) => /* handle error */,
    /// }
    /// ```
    pub fn fetch_assignments(&self) -> Result<Vec<Assignment>, Box<dyn Error>> {
        {
            let assignments_cache = self.info.assignments_cache.lock().unwrap();
            if !assignments_cache.is_empty() {
                return Ok(assignments_cache.clone());
            }
        }
        match canvas::fetch_assignments(self) {
            Ok(assignments) => {
                let mut assignments_cache = self.info.assignments_cache.lock().unwrap();
                assignments_cache.extend(assignments.clone());
                Ok(assignments_cache.to_vec())
            }
            Err(e) => Err(e),
        }
    }

    pub fn choose_assignment(
        &self,
        text: Option<&str>,
        assignments: Option<Vec<Assignment>>,
    ) -> Option<(Vec<Assignment>, usize)> {
        let mut assignments = assignments;
        loop {
            let mut menu_str = Vec::new();
            let mut menu_assignment = Vec::new();

            let assignment_list = match assignments {
                Some(assignment_list) => assignment_list,
                None => {
                    println!("Fetching assignments...");
                    match self.fetch_assignments() {
                        Ok(assignments) => assignments,
                        Err(_) => {
                            eprintln!("Failed to download assignments from Canvas");
                            exit(1);
                        }
                    }
                }
            };

            for assignment in assignment_list.iter() {
                menu_str.push(assignment.info.name.clone());
                menu_assignment.push(assignment);
            }

            // Add REFRESH THIS LIST at the end of the list
            menu_str.push("REFRESH THIS LIST".to_string());

            // Add EXIT at the end of the list
            menu_str.push("EXIT".to_string());

            let prompt: &str = text.unwrap_or_else(|| "Choose a assignment:");

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt)
                .items(&menu_str)
                .default(0)
                .interact()
                .unwrap();

            if selection == menu_str.len() - 1 {
                return None;
            }

            if selection == menu_str.len() - 2 {
                assignments = None;
                continue;
            }
            return Some((assignment_list, selection));
        }
    }

    /// Updates the score of a student's assignment submission.
    ///
    /// Sends an HTTP PUT request to the Canvas API to update the score for a specific assignment
    /// submission by a student. Handles request construction, execution, and authentication using
    /// Canvas API credentials.
    ///
    /// Arguments:
    /// - `client`: HTTP client for executing requests.
    /// - `assignment_id`: ID of the assignment.
    /// - `student_id`: ID of the student.
    /// - `new_score`: New score to be set, or `None` to clear the existing score.
    ///
    /// Returns:
    /// - `Result<(), Box<dyn std::error::Error>>`: Success or an error detailing any issues encountered.
    ///
    /// Example:
    /// ```
    /// let client = reqwest::blocking::Client::new();
    /// let course = Course { /* ... */ };
    /// match course.update_assignment_score(&client, assignment_id, student_id, new_score) {
    ///     Ok(_) => /* handle success */,
    ///     Err(e) => /* handle error */,
    /// }
    /// ```
    pub fn update_assignment_score(
        &self,
        assignment_id: u64,
        student_id: u64,
        new_score: Option<f64>,
    ) -> Result<(), Box<dyn Error>> {
        let result = canvas::update_assignment_score(
            &self.info.canvas_info,
            self.info.id,
            assignment_id,
            student_id,
            new_score,
        );
        if result.is_ok() {
            self.clear_cache();
        }
        result
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
    /// let client = reqwest::blocking::Client::new();
    /// let course = Course { /* ... */ };
    /// match course.add_file_comment(&client, assignment_id, student_id, Some("path/to/file.pdf"), "Great work!") {
    ///     Ok(_) => /* handle success */,
    ///     Err(e) => /* handle error */,
    /// }
    /// ```
    pub fn comment_with_file(
        &self,
        client: &Client,
        assignment_id: u64,
        student_id: u64,
        file_path: Option<&str>,
        comment_text: &str,
    ) -> Result<(), Box<dyn Error>> {
        let result = canvas::comment_with_file(
            client,
            &self.info.canvas_info,
            self.info.id,
            assignment_id,
            student_id,
            file_path,
            comment_text,
        );
        if result.is_ok() {
            self.clear_cache();
        }
        result
    }

    pub fn comment_with_binary_file(
        &self,
        assignment_id: u64,
        student_id: u64,
        file_name: Option<&str>,
        file_content: Option<&Vec<u8>>,
        comment_text: &str,
    ) -> Result<(), Box<dyn Error>> {
        let result = canvas::comment_with_binary_file(
            &self.info.canvas_info.client,
            &self.info.canvas_info,
            self.info.id,
            assignment_id,
            student_id,
            file_name,
            file_content,
            comment_text,
        );
        if result.is_ok() {
            self.clear_cache();
        }
        result
    }

    pub fn create_assignment(&self, assignment_name: &str) -> Result<(), Box<dyn Error>> {
        let result =
            canvas::create_assignment(&self.info.canvas_info, self.info.id, assignment_name);
        if result.is_ok() {
            self.clear_cache();
        }
        result
    }

    pub fn create_announcement(&self, title: &str, message: &str) -> Result<(), Box<dyn Error>> {
        let result =
            canvas::create_announcement(&self.info.canvas_info, self.info.id, title, message);
        if result.is_ok() {
            self.clear_cache();
        }
        result
    }

    /// Loads the information of a specific course from the Canvas LMS based on the course ID.
    ///
    /// This function uses Canvas API credentials and makes a request to retrieve the details
    /// of a course by the given ID. It returns a `Course` object containing all relevant course information,
    /// or an error if the operation fails.
    ///
    /// # Parameters
    ///
    /// - `id`: The unique identifier of the course in the Canvas LMS.
    ///
    /// # Returns
    ///
    /// Returns a `Result<Course, Box<dyn Error>>`, where:
    /// - `Ok(Course)` contains the successfully loaded course data.
    /// - `Err(Box<dyn Error>)` contains an error message in case of connection failure or invalid credentials.
    ///
    /// # Example
    ///
    /// ```
    /// let course_id = 12345;
    /// match new_course_from_course_id(course_id) {
    ///     Ok(course) => println!("Course loaded successfully: {:?}", course),
    ///     Err(e) => eprintln!("Error loading course: {}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function returns errors in the following cases:
    /// - Failed connection to Canvas LMS.
    /// - Invalid credentials for the Canvas API.
    ///
    /// The function uses `Canvas::fetch_single_course_with_credentials` to make the API call
    /// and handle authentication, transforming the received JSON data into a `Course` object.
    pub fn get_course_from_course_id(id: u64) -> Result<Course, Box<dyn Error>> {
        // Pegue as credenciais do Canvas
        let credentials = CanvasCredentials::credentials();

        // Busque o curso com o ID fornecido
        match Canvas::fetch_single_course_with_credentials(&credentials, id) {
            CanvasResultSingleCourse::Ok(course) => Ok(course),
            CanvasResultSingleCourse::ErrConnection(msg) => {
                eprintln!("Erro de conexão: {}", msg);
                Err(format!("Erro de conexão: {}", msg).into())
            }
            CanvasResultSingleCourse::ErrCredentials(msg) => {
                eprintln!("Erro de credenciais: {}", msg);
                Err(format!("Erro de credenciais: {}", msg).into())
            }
        }
    }

    // Retrieves a specific assignment from the course based on the assignment ID.
    ///
    /// This method makes an API call to fetch the details of a particular assignment in the course
    /// by the provided assignment ID. It returns the `Assignment` object containing the assignment's
    /// details or an error if the assignment is not found or if the request fails.
    ///
    /// # Parameters
    ///
    /// - `id`: The unique identifier of the assignment in the Canvas LMS.
    ///
    /// # Returns
    ///
    /// Returns a `Result<Assignment, Box<dyn Error>>`, where:
    /// - `Ok(Assignment)` contains the successfully loaded assignment data.
    /// - `Err(Box<dyn Error>)` contains an error message in case the assignment is not found or any issue occurs.
    ///
    /// # Example
    ///
    /// ```
    /// let course = Course { /* initialized */ };
    /// let assignment_id = 12345;
    /// match course.get_assignment_from_assignment_id(assignment_id) {
    ///     Ok(assignment) => println!("Assignment loaded successfully: {:?}", assignment),
    ///     Err(e) => eprintln!("Error loading assignment: {}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns errors if the assignment is not found or if there is a failure in the API request.
    pub fn get_assignment_from_assignment_id(&self, id: u64) -> Result<Assignment, Box<dyn Error>> {
        // Fetch all assignments for the course
        let assignments = self.fetch_assignments()?;

        // Try to find the assignment with the given ID
        match assignments
            .into_iter()
            .find(|assignment| assignment.info.id == id)
        {
            Some(assignment) => Ok(assignment), // Assignment found
            None => Err(format!("Assignment with id {} not found", id).into()), // Assignment not found
        }
    }
}

/// Structure to store course name details.
///
/// Contains fields to represent various parts of a course name,
/// including subject, period, class, course code, shift, year, semester,
/// abbreviated name, class details, and the final result.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CourseNameDetails {
    pub subject: String,
    pub period: String,
    pub class: String,
    pub course_code: String,
    pub shift: String,
    pub year: String,
    pub semester: String,
    pub abbreviated_name: String,
    pub canvas_full_name: String,
}

/// Parses the course name string from Canvas and extracts structured details.
///
/// This function applies regex matching to interpret the course name format commonly
/// used in Canvas. It captures key information such as the course's discipline, period,
/// group, and semester details and packs them into a CourseNameDetails for easy access.
///
/// The function is designed to recognize and extract a standardized set of academic
/// identifiers from a course name string, which can vary in format.
///
/// # Arguments
///
/// * `canvas_name` - A string slice representing the full name of a course as provided by Canvas.
///
/// # Returns
///
/// An `Option<CourseNameDetails>` where the keys are elements like 'discipline', 'period',
/// 'group', etc., and the values are the corresponding details extracted from the course name.
/// Returns `None` if the course name does not match the expected pattern.
#[allow(dead_code)]
pub fn parse_course_name(canvas_name: &str, cavas_full_name: &str) -> Option<CourseNameDetails> {
    let regex = Regex::new(r"(?m)\[([^\.\[\]]+)\.([^\.\[\]]+)\.([^\.\[\]]+)\.([^\.\[\]]+)\.([^\.\[\]]+)\.([^\.\[\]]+)\.([^\.\[\]]+)\]").unwrap();
    let captures = match regex.captures(canvas_name) {
        Some(caps) => caps,
        None => {
            return None;
        }
    };

    // let (result, curso) = ajusta_nome_curso(canvas_name)?;
    let course_details = CourseNameDetails {
        subject: captures[1].to_string(),
        course_code: captures[2].to_string(),
        class: captures[3].to_string(),
        period: captures[4].to_string(),
        shift: captures[5].to_string(),
        year: captures[6].to_string(),
        semester: captures[7].to_string(),
        abbreviated_name: format!(
            "{}.{}.{}.{}.{}.{}.{}",
            &captures[1],
            &captures[2],
            &captures[3],
            &captures[4],
            &captures[5],
            &captures[6],
            &captures[7]
        ),
        canvas_full_name: cavas_full_name.to_string(),
    };
    Some(course_details)
}

/// Abbreviates a course name based on specific rules.
///
/// This function takes a course name and processes it to create an abbreviation.
/// The rules for abbreviation are as follows:
/// - Parts of the name with less than 4 characters are excluded.
/// - Each remaining part is truncated or padded to 6 characters.
/// - All characters are converted to lowercase, except for the first character which is capitalized.
/// - If there is only one part, the first 6 characters of this part are used.
/// - If there are two parts, the first 3 characters of each part are concatenated.
/// - If there are three or more parts, the first 2 characters of the first two parts and the first 2 characters
///   of the last part are concatenated.
///
/// # Arguments
///
/// * `name` - A string slice that holds the name of the course.
///
/// # Returns
///
/// A `String` representing the abbreviated course name.
pub fn abbreviate_course_name(name: &str) -> String {
    let parts: Vec<String> = name
        .split_whitespace()
        .filter(|&p| p.len() >= 4)
        .map(|p| {
            let mut part = p.to_lowercase();
            part.replace_range(0..1, &part[0..1].to_uppercase());
            if part.len() > 6 {
                part.truncate(6);
            }
            part
        })
        .collect();

    match parts.len() {
        0 => String::new(),
        1 => parts[0].chars().take(6).collect(),
        2 => format!("{}{}", &parts[0][0..3], &parts[1][0..3]),
        _ => format!(
            "{}{}{}",
            &parts[0][0..2],
            &parts[1][0..2],
            &parts.last().unwrap()[0..2]
        ),
    }
}
