// Necessary imports from standard and external crates.
use std::sync::Arc;
//use lazy_static::lazy::Lazy;
use serde::{Deserialize, Serialize};
use crate::{CanvasCredentials};
use crate::connection::{send_http_request, HttpMethod};
use crate::student::{Student, StudentInfo};
use crate::assignment::{Assignment, AssignmentInfo};

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
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CourseInfo {
    pub id: u64,
    pub name: String,
    pub course_code: String,
    #[serde(skip)]
    pub canvas_info: Arc<CanvasCredentials>,
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
            match send_http_request(
                client,
                HttpMethod::Get, // Supondo que HttpMethod::Get é um enum definido em algum lugar
                &url,
                &self.info.canvas_info,
                converted_params, // Passando o Vec<(String, String)> diretamente
            ) {
                Ok(response) => {
                    if response.status().is_success() {
                        let students_page: Vec<serde_json::Value> = response.json()?;
                        if students_page.is_empty() {
                            break; // Sai do loop se não há mais estudantes
                        }
                        all_students.extend(students_page.into_iter().filter_map(|student| {
                            Course::convert_json_to_student(&self.info, &student)
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

    /// Converts a JSON object to a `Student` structure.
    ///
    /// Parses a JSON representation of a student from the Canvas API into a `Student` object.
    /// Extracts student ID, name, and email and associates it with course information.
    ///
    /// Arguments:
    /// - `course_info`: Reference to the course's information.
    /// - `student`: JSON object representing the student.
    ///
    /// Returns:
    /// - `Option<Student>`: A `Student` object if conversion is successful, or `None` if not.
    ///
    /// Example:
    /// ```
    /// let course_info = Arc::new(CourseInfo { /* ... */ });
    /// let student_json = /* ... */;
    /// let student = convert_json_to_student(&course_info, &student_json);
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

            match send_http_request(
                client,
                HttpMethod::Get,
                &url,
                &self.info.canvas_info,
                params,
            ) {
                Ok(response) => {
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
                        ))); // Retorna um erro
                    }
                }
                Err(e) => {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to fetch assignments with error: {}", e),
                    ))); // Retorna um erro
                }
            }
        }
        Ok(all_assignments)
    }

    /// Converts a JSON object into an `Assignment` structure.
    ///
    /// Transforms a JSON representation of an assignment into an `Assignment` object. Retrieves key
    /// details such as ID, name, and description, linking them with the course information.
    ///
    /// Arguments:
    /// - `course_info`: Reference to the course's information.
    /// - `assignment`: JSON object representing the assignment.
    ///
    /// Returns:
    /// - `Option<Assignment>`: An `Assignment` object if conversion is successful, or `None` if not.
    ///
    /// Example:
    /// ```
    /// let course_info = Arc::new(CourseInfo { /* ... */ });
    /// let assignment_json = /* ... */;
    /// let assignment = convert_json_to_assignment(&course_info, &assignment_json);
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

        match send_http_request(
            client,
            HttpMethod::Put(body), // Use HttpMethod::Put enum variant
            &url,
            &self.info.canvas_info,
            Vec::new(), // PUT request does not need params
        ) {
            Ok(response) => match response.status().is_success() {
                true => Ok(()),
                false => Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to update score with status: {}", response.status()),
                ))),
            },
            Err(e) => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to update score with error: {}", e),
            ))),
        }
    }
}
