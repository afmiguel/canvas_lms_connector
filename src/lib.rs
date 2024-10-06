//! # Canvas API Integration Library
//!
//! This Rust library provides functionalities for interacting with the Canvas Learning Management System (LMS) API.
//! It simplifies tasks like retrieving course information, managing student data, and processing assignments and submissions.
//! The library utilizes the `reqwest` crate for HTTP requests and incorporates concurrency control for efficient request handling.
//!
//! ## Core Features
//!
//! - **Authentication and Configuration:** Handles Canvas API credentials, supporting both file-based and system keyring storage.
//! - **Course Management:** Facilitates access to course information, enabling users to interact with course details.
//! - **Student Management:** Provides functionalities to manage student data within courses.
//! - **Assignments and Submissions Handling:** Allows for retrieval and updating of assignment information and student submissions.
//!
//! ## Usage
//!
//! To use this library, add it as a dependency in your `Cargo.toml`. Use the provided structures and functions
//! to interact with the Canvas API as per your application's requirements.
//!
//! ```toml
//! [dependencies]
//! canvas_lms_connector = "0.1"
//! ```
//!
//! After adding the library as a dependency, you can use its features in your Rust application.
//!
//! The primary functions are `fetch_courses_with_credentials` and `fetch_single_course_with_credentials`.
//! - `fetch_courses_with_credentials` retrieves a list of courses using provided Canvas API credentials.
//! - `fetch_single_course_with_credentials` fetches details of a specific course using the given credentials.
//!
//! Both functions require a reference to `CanvasCredentials`, which contain the necessary API URL and token.
//! They return results encapsulated in `CanvasResultCourses` or `CanvasResultSingleCourse` enums,
//! representing either successful data retrieval or an error (connection or credential issues).
//!
//! ### Examples
//!
//! Fetching a list of courses:
//! ```rust
//! let canvas_credentials = CanvasCredentials { /* ... */ };
//! match Canvas::fetch_courses_with_credentials(&canvas_credentials) {
//!     CanvasResultCourses::Ok(courses) => println!("Courses: {:?}", courses),
//!     CanvasResultCourses::ErrConnection(err) => eprintln!("Connection error: {}", err),
//!     CanvasResultCourses::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
//! }
//! ```
//!
//! Fetching a specific course by ID:
//! ```rust
//! let canvas_credentials = CanvasCredentials { /* ... */ };
//! let course_id = 123;
//! match Canvas::fetch_single_course_with_credentials(&canvas_credentials, course_id) {
//!     CanvasResultSingleCourse::Ok(course) => println!("Course: {:?}", course),
//!     CanvasResultSingleCourse::ErrConnection(err) => eprintln!("Connection error: {}", err),
//!     CanvasResultSingleCourse::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
//! }
//! ```
mod assignment; // Manages assignments within Canvas courses.
pub mod canvas;
mod connection; // Manages HTTP connections and requests to the Canvas API.
pub mod course; // Contains functionalities related to Canvas courses.
pub mod credentials; // Handles the storage and retrieval of Canvas API credentials.
pub mod rubric_downloaded;
pub mod rubric_submission;
mod student; // Deals with operations related to students in Canvas courses.
mod submission; // Handles submissions for assignments in Canvas.

// Exports key structures for external use.
pub use assignment::{Assignment, AssignmentInfo, GetSubmissionFromSubmissionIdCache};
pub use canvas::{Canvas, CanvasResultCourses, CanvasResultSingleCourse};
pub use course::{Course, CourseInfo};
pub use credentials::CanvasCredentials;
pub use student::{Student, StudentInfo};
pub use submission::{Submission, SubmissionType};

// #[cfg(test)]
// mod tests {
//     // Test implementations for library functionalities.
//     // Example:
//     // #[test]
//     // fn test_fetch_courses() {
//     //     let credentials = CanvasCredentials::load_from_file("path/to/credentials");
//     //     let result = Canvas::fetch_courses_with_credentials(&credentials);
//     //     assert!(matches!(result, CanvasResultCourses::Ok(_)));
//     // }
// }

#[cfg(test)]
mod tests {
    use crate::canvas::create_rubric;
    use crate::rubric_submission::{
        CanvasRubricSubmission, CriterionSubmission, RatingSubmission, RubricAssociationSubmission,
        RubricSubmissionDetails,
    };
    use crate::CanvasCredentials;
    use reqwest::blocking::Client;
    use std::collections::HashMap;

    #[test]
    fn test_create_rubric() {
        // Defining the rubric for the test, based on the simplified structure
        let rubric = CanvasRubricSubmission {
            rubric: RubricSubmissionDetails {
                title: "My New Rubric".to_string(),
                criteria: {
                    let mut criteria_map = HashMap::new();
                    criteria_map.insert(
                        "1".to_string(),
                        CriterionSubmission {
                            description: "Stakeholder Identification".to_string(),
                            criterion_use_range: Some(false),
                            ratings: {
                                let mut ratings_map = HashMap::new();
                                ratings_map.insert(
                                    "1".to_string(),
                                    RatingSubmission {
                                        description: "All stakeholders identified".to_string(),
                                        points: 5.0,
                                    },
                                );
                                ratings_map.insert(
                                    "2".to_string(),
                                    RatingSubmission {
                                        description: "Few stakeholders identified".to_string(),
                                        points: 0.0,
                                    },
                                );
                                ratings_map
                            },
                        },
                    );
                    criteria_map
                },
            },
            rubric_association: RubricAssociationSubmission {
                association_type: "Course".to_string(),
                association_id: 43689,
                use_for_grading: false,
            },
        };

        // Define Canvas credentials for the test
        let credentials = CanvasCredentials {
            url_canvas: "https://pucpr.beta.instructure.com/api/v1".to_string(),
            token_canvas: "20746~JhvKCm9LGeQ7zf4yKXn3YmPvtK6LFrayT2La9VNZ2vE8QHWHBWQJxcFHY6xKBYeh"
                .to_string(),
            client: Client::new(),
        };

        // Call the function that creates the rubric
        match create_rubric(&credentials, 43689, &rubric) {
            Ok(_) => println!("Rubric created successfully!"),
            Err(e) => panic!("Error creating rubric: {}", e),
        }
    }
}
