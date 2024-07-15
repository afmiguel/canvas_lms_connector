// Import necessary crates and modules
use std::sync::Arc;
use chrono::{DateTime, Utc};
// use std::thread::sleep;
use serde::{Deserialize, Serialize};
use crate::{canvas, CourseInfo, Student};
use crate::submission::Submission;
use serde_json::Value;

// use crate::student::Student;
// use crate::canvas::Canvas;
// use crate::connection::{HttpMethod, send_http_request};

/// Structure to hold detailed information about an assignment in the Canvas system.
///
/// This struct is essential for representing an assignment in the context of the Canvas Learning Management System (LMS).
/// It includes several fields to store the key details of an assignment, and a shared reference to the `CourseInfo` structure,
/// connecting the assignment with its associated course and enabling API interactions.
///
/// Fields:
/// - `id`: Unique identifier for the assignment in the Canvas system.
/// - `name`: The name of the assignment.
/// - `description`: Optional detailed description of the assignment.
/// - `course_info`: A thread-safe reference (`Arc`) to the `CourseInfo` struct, which contains course-specific details and API credentials.
///
/// The use of `Arc<CourseInfo>` ensures that the `CourseInfo` data can be safely shared and accessed across multiple threads,
/// which is crucial for concurrent processing in web applications or multi-threaded environments.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AssignmentInfo {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    #[serde(skip)]
    pub course_info: Arc<CourseInfo>,
}

/// High-level structure representing an assignment within the Canvas Learning Management System.
///
/// This struct serves as a wrapper around the `AssignmentInfo` struct, providing a more abstracted representation
/// of an assignment. It is particularly useful in scenarios where assignment-related operations are performed,
/// such as fetching, updating, or displaying assignment details. The use of `Arc<AssignmentInfo>` allows for efficient
/// sharing and management of `AssignmentInfo` data across different components or threads of an application.
///
/// Fields:
/// - `info`: A thread-safe, shared reference (`Arc`) to an `AssignmentInfo` instance. This encapsulates all the
///   detailed information about the assignment, such as its ID, name, description, and related course information.
///
/// The `Assignment` struct is a fundamental part of any application that interacts with the Canvas API for assignment-related
/// functionalities, simplifying the handling of assignments and their associated data.
#[derive(Debug, Clone)]
pub struct Assignment {
    pub info: Arc<AssignmentInfo>,
}

impl Assignment {
    pub fn fetch_submissions(&self, students: &Vec<Student>) -> Result<Vec<Submission>, Box<dyn std::error::Error>> {
        let client = &reqwest::blocking::Client::new();
        match canvas::get_all_submissions(client, self.info.course_info.canvas_info.as_ref(), self.info.course_info.id, self.info.id) {
            Ok(submissions_value) => {
                let submissions = submissions_value
                    .iter()
                    .filter_map(|j| {
                        Assignment::convert_json_to_submission(students, j)
                    })
                    .collect::<Vec<_>>();

                Ok(submissions)
            }
            Err(e) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to fetch submissions with error: {} (a)", e),
                )));
            }
        }
    }

    fn convert_json_to_submission(
        students: &Vec<Student>,
        j: &Value,
    ) -> Option<Submission> {
        // Encontra qual é o estudante respectivo user_id
        for student in students {
            match j["user_id"].as_u64() {
                Some(user_id) => {
                    if student.info.id == user_id {
                        return Some(Submission {
                            id: j["id"].as_u64().unwrap(),                             // Submission's unique identifier
                            assignment_id: j["assignment_id"].as_u64().unwrap(),                  // Assignment's unique identifier
                            score: j["score"].as_f64(),                  // Graded score, optional
                            submitted_at: j["submitted_at"].as_str().map(|s| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)), // Submission timestamp, optional
                            student: student.info.clone(),
                        });
                    }
                }
                None => {}
            }
        }
        None
    }
}