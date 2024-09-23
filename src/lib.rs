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
mod connection; // Manages HTTP connections and requests to the Canvas API.
pub mod credentials; // Handles the storage and retrieval of Canvas API credentials.
pub mod course; // Contains functionalities related to Canvas courses.
mod student; // Deals with operations related to students in Canvas courses.
mod assignment; // Manages assignments within Canvas courses.
mod submission; // Handles submissions for assignments in Canvas.
pub mod canvas;
mod rubric;

// Exports key structures for external use.
pub use credentials::CanvasCredentials;
pub use course::{Course, CourseInfo};
pub use student::{Student, StudentInfo};
pub use assignment::{Assignment, AssignmentInfo};
pub use submission::{Submission, SubmissionType};
pub use canvas::{Canvas, CanvasResultCourses, CanvasResultSingleCourse};

#[cfg(test)]
mod tests {
    // Test implementations for library functionalities.
    // Example:
    // #[test]
    // fn test_fetch_courses() {
    //     let credentials = CanvasCredentials::load_from_file("path/to/credentials");
    //     let result = Canvas::fetch_courses_with_credentials(&credentials);
    //     assert!(matches!(result, CanvasResultCourses::Ok(_)));
    // }
}
