//! # Canvas API Integration Library
//!
//! This library provides a range of functionalities for interacting with the Canvas Learning Management System API.
//! It supports operations such as course retrieval, student management, and handling of assignments and submissions.
//! The library uses the `reqwest` library for making HTTP requests and implements concurrency control to limit the number of simultaneous requests.
//!
//! ## Core Features
//!
//! - **Authentication and Configuration:** Load Canvas API credentials from configuration files or the system keyring.
//! - **Course Management:** Retrieve information about courses available to an authenticated user.
//! - **Student Management:** Fetch students enrolled in specific courses.
//! - **Assignments and Submissions Handling:** Retrieve and update assignments and student submissions.
//!
//! ## Usage
//!
//! To use this library, add it as a dependency in your `Cargo.toml`. Then, utilize the structures and functions
//! provided to interact with the Canvas API as needed for your application.
//!
//! ```toml
//! [dependencies]
//! canvas_lms_connector = "0.1"
//! ```
//!
//! After adding the dependency, you can start using the library's features in your code.
//!
//! The entry point is the function `fetch_courses`, which can be used to retrieve courses.
//! The function takes a closure as an argument, which is used to specify the type of course retrieval.
//! The closure should take a reference to `CanvasInfo` and return a `CanvasResult`.
//! The `CanvasResult` enum encapsulates the possible outcomes of the operation, including successful
//! retrieval of courses or various types of errors.
//! The `fetch_courses` function handles credential management and error handling, while the closure
//! specifies the specific course retrieval logic.
//! This approach allows for flexibility in the way courses are retrieved, while ensuring that the
//! authentication and error handling are handled in a consistent way.
//!
//! The following example demonstrates the usage of `fetch_courses` to retrieve courses using
//! credentials stored in the system keyring.
//!
//! # Examples
//!
//! ```
//! match Canvas::fetch_courses(|credential| Canvas::fetch_courses_with_credentials(credential)) {
//!     CanvasResult::Ok(courses) => println!("Courses fetched: {:?}", courses),
//!     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
//!     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
//! }
//! ```
//! or
//!  ```
//! let course_id = 123;
//! match Canvas::fetch_courses(|credential| Canvas::fetch_single_course_with_credentials(credential, course_id)) {
//!     CanvasResult::Ok(courses) => println!("Courses fetched: {:?}", courses),
//!     CanvasResult::ErrConnection(err) => eprintln!("Connection error: {}", err),
//!     CanvasResult::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
//! }
//! ```
mod connection;
pub mod credentials;
mod course;
mod student;
mod assignment;
mod submission;
mod canvas;

pub use credentials::CanvasCredentials;
pub use course::{Course, CourseInfo};
pub use student::{Student, StudentInfo};
pub use assignment::{Assignment, AssignmentInfo};
pub use submission::{Submission};
pub use canvas::{Canvas, CanvasResultCourses, CanvasResultSingleCourse};

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn it_works() {}
}
