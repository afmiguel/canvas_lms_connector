// Import necessary crates and modules
use std::sync::Arc;
use chrono::{DateTime, Utc}; // chrono crate is used for handling date and time
use serde::{Deserialize, Serialize}; // serde for serialization and deserialization
use crate::StudentInfo; // StudentInfo struct from the current crate

/// Structure representing a student's submission for an assignment in the Canvas Learning Management System.
///
/// This struct provides a detailed view of a student's submission, capturing key aspects like the submission's ID,
/// the associated assignment ID, the score (if already graded), and the timestamp of submission. It also includes
/// a reference to the `StudentInfo` struct to establish a direct link to the student who made the submission.
///
/// Fields:
/// - `id`: Unique identifier for the submission within the Canvas system.
/// - `assignment_id`: Identifier of the assignment this submission is related to.
/// - `score`: Optional field that contains the score if the submission has been graded.
/// - `submitted_at`: Optional field indicating the date and time when the submission was made, using UTC timezone.
/// - `student`: Thread-safe shared reference (`Arc`) to `StudentInfo`, which contains data about the student.
///
/// The use of `Arc<StudentInfo>` is crucial for concurrent access and efficient memory management when the same student's 
/// information is accessed from multiple points in the program. This struct is essential for functionalities that involve 
/// tracking and assessing student performance, especially in digital learning environments like Canvas.
///
/// Examples of related functions include `fetch_submissions_for_assignments` and `fetch_assignments_and_latest_submissions`,
/// which likely utilize this struct to represent and handle student submissions.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Submission {
    pub id: u64, // Submission's unique identifier
    pub assignment_id: u64, // Assignment's unique identifier
    pub score: Option<f64>, // Graded score, optional
    pub submitted_at: Option<DateTime<Utc>>, // Submission timestamp, optional
    #[serde(skip)] // Skipped during serialization/deserialization
    pub student: Arc<StudentInfo>, // Shared reference to student information
}
