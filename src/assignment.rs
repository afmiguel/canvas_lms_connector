use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::CourseInfo;


/// Contains detailed information about an assignment in the Canvas system.
///
/// This structure is used to store and manage data specific to an assignment within a Canvas course.
/// It includes key details such as the assignment's unique identifier, its name, and a description (if provided).
/// Additionally, the structure holds a reference to `CourseInfo`, linking the assignment to its respective
/// course context and providing necessary information for API interactions.
///
/// # Fields
///
/// - `id`: The unique identifier of the assignment in the Canvas system.
/// - `name`: The name of the assignment.
/// - `description`: An optional description of the assignment.
/// - `course_info`: A shared reference (`Arc`) to `CourseInfo` containing course-specific details and API credentials.
///
/// The `AssignmentInfo` structure is central to operations involving assignment data in the Canvas LMS, such as
/// fetching, updating, and managing assignments and their related activities.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AssignmentInfo {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    #[serde(skip)]
    pub course_info: Arc<CourseInfo>,
}

/// Represents an assignment within the Canvas Learning Management System.
///
/// This structure encapsulates the details of a specific assignment in a Canvas course. It primarily
/// consists of `AssignmentInfo`, which contains essential information like the assignment's ID, name,
/// and optional description. The `Assignment` struct serves as a high-level representation of an assignment
/// in Canvas, facilitating the access to and manipulation of assignment-related data in various operations
/// and API interactions.
///
/// # Fields
///
/// - `info`: A shared reference (`Arc`) to an `AssignmentInfo` instance containing detailed information
///   about the assignment.
///
/// The `Assignment` struct is a key component in applications interacting with the Canvas API, providing a
/// convenient and unified way to handle assignment-related data.
#[derive(Debug, Clone)]
pub struct Assignment {
    pub info: Arc<AssignmentInfo>,
}
