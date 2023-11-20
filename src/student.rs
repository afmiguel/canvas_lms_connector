use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::CourseInfo;
use crate::assignment::{Assignment, AssignmentInfo};
use crate::connection::{HttpMethod, send_http_request};
use crate::submission::Submission;

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
            );
            interaction();
            match response {
                Ok(response) => {
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
                Err(e) => {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to fetch submissions with error: {}", e),
                    )));
                }
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
