// Import necessary crates and modules
use crate::assignment::{Assignment, AssignmentInfo};
use crate::canvas;
use crate::submission::Submission;
use crate::CourseInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Structure for storing and managing student data in the Canvas system.
///
/// This struct encapsulates key information about a student, particularly relevant in the context of a Canvas course.
/// It holds essential details like the student's identifier, name, and email, and also maintains a link to course-specific
/// information and API credentials through `CourseInfo`.
///
/// Fields:
/// - `id`: The unique identifier of the student in Canvas.
/// - `name`: The student's full name.
/// - `email`: The student's email address.
/// - `course_info`: A thread-safe reference (`Arc`) to the course information and API credentials (`CourseInfo`).
///
/// The struct is essential for various student-related operations in the Canvas API, such as retrieving student details,
/// managing course enrollments, etc.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StudentInfo {
    pub id: u64,
    pub name: String,
    pub email: String,
    #[serde(skip)]
    pub course_info: Arc<CourseInfo>,
}

/// High-level representation of a student in the Canvas Learning Management System.
///
/// This struct acts as a wrapper around `StudentInfo`, providing a streamlined way to manage and access student data.
/// It is particularly useful for operations that involve student information, such as fetching grades, submissions,
/// and other student-specific details.
///
/// Fields:
/// - `info`: A shared (`Arc`) reference to `StudentInfo` that contains the detailed information about the student.
///
/// This struct is central to handling student data efficiently, especially in scenarios requiring concurrent access
/// to the same student information.
#[derive(Debug, Clone)]
pub struct Student {
    pub info: Arc<StudentInfo>,
}

/// Implementation of the `Student` struct, providing methods for student-specific interactions in the Canvas system.
///
/// These methods offer functionalities such as fetching submissions for assignments and updating assignment scores,
/// tailored to individual students. They utilize the student's information and API access credentials for making
/// authenticated requests to the Canvas API.
impl Student {
    /// Fetches submissions for specified assignments for this student.
    ///
    /// Queries the Canvas API to retrieve submissions made by this student for a given set of assignments.
    /// It requires authenticated access, using the student's ID and course API credentials.
    ///
    /// Type Parameters:
    /// - `F`: A function trait for additional interactions or side effects.
    ///
    /// Arguments:
    /// - `client`: A HTTP client for executing requests.
    /// - `assignment_ids`: IDs of the assignments for which submissions are to be fetched.
    /// - `interaction`: A closure for any additional processing or side effects.
    ///
    /// Returns:
    /// - `Result<Vec<Submission>, Box<dyn std::error::Error>>`: A result containing either a list of submissions
    ///   or an error encapsulating any issues encountered during the API call.
    pub fn fetch_submissions_for_assignments<F>(
        &self,
        assignments_info: &Vec<Arc<AssignmentInfo>>,
        interaction: F,
    ) -> Result<Vec<Submission>, Box<dyn std::error::Error>>
    where
        F: Fn(),
    {
        canvas::fetch_submissions_for_assignments(
            self.info.course_info.canvas_info.as_ref(),
            &self.info,
            &self.info.course_info.fetch_students()?,
            assignments_info,
            interaction,
        )
    }

    /// Retrieves assignments and their latest submissions for the student.
    ///
    /// Fetches assignment details along with the most recent submissions made by the student. This method
    /// provides a comprehensive view of a student's progress and submissions for specific assignments.
    ///
    /// Type Parameters:
    /// - `F`: A closure type for additional interactions.
    ///
    /// Arguments:
    /// - `client`: The HTTP client used for making requests.
    /// - `assignments`: A shared reference to a collection of `Assignment` objects.
    /// - `interaction`: A closure for additional operations during processing.
    ///
    /// Returns:
    /// - `Result<HashMap<u64, (Arc<AssignmentInfo>, Option<Submission>)>, Box<dyn std::error::Error>>`: A result
    ///   containing a map of assignment IDs to tuples of `AssignmentInfo` and the latest `Submission`, or an error.
    pub fn fetch_assignments_and_latest_submissions<F>(
        &self,
        assignments: Arc<Vec<Assignment>>,
        interaction: F,
    ) -> Result<HashMap<u64, (Arc<AssignmentInfo>, Option<Submission>)>, Box<dyn std::error::Error>>
    where
        F: Fn(),
    {
        let mut assignments_info: Vec<Arc<AssignmentInfo>> = Vec::new();
        for assignment in assignments.iter() {
            assignments_info.push(assignment.info.clone());
        }

        let submissions = self.fetch_submissions_for_assignments(&assignments_info, interaction)?;

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
