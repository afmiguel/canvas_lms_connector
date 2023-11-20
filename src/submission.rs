use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::StudentInfo;


/// Represents a student's submission for an assignment in the Canvas Learning Management System.
///
/// This structure contains detailed information about a specific submission made by a student for an assignment.
/// It includes the submission's unique identifier, the assignment ID it belongs to, the score (if graded),
/// and the timestamp when the submission was made. Additionally, it holds a reference to the `StudentInfo` of
/// the student who made the submission, linking it directly to the student's data.
///
/// # Fields
///
/// - `id`: The unique identifier of the submission in the Canvas system.
/// - `assignment_id`: The ID of the assignment to which this submission corresponds.
/// - `score`: An optional field containing the score of the submission, if graded.
/// - `submitted_at`: An optional field indicating the date and time of submission.
/// - `student`: A shared reference (`Arc`) to `StudentInfo`, linking the submission to the student.
///
/// The structure plays a vital role in the digital workflow of assignments, enabling effective tracking
/// and assessment of student performance in the Canvas environment.
///
/// See also: fetch_submissions_for_assignments, fetch_assignments_and_latest_submissions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Submission {
    pub id: u64,
    pub assignment_id: u64,
    pub score: Option<f64>,
    pub submitted_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub student: Arc<StudentInfo>,
}