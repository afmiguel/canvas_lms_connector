use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::{CanvasCredentials};
use crate::connection::{send_http_request, HttpMethod};
use crate::student::{Student, StudentInfo};
use crate::assignment::{Assignment, AssignmentInfo};

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
    pub canvas_info: Arc<CanvasCredentials>,
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
