# Canvas LMS Connector

## Overview

This documentation introduces the Canvas LMS Connector, a Rust library designed to facilitate interaction with the Canvas Learning Management System (LMS) API. Crafted to integrate Rust applications with the features of Canvas, this library offers an effective solution for data manipulation in educational environments.

The focus of Canvas LMS Connector (`canvas_lms_connector`) is to provide an accessible interface for developers seeking to interact with the Canvas API, encompassing everything from automating administrative processes to supporting educational initiatives. The library caters to various needs, ranging from course management to the development of customized applications.

### Key Features

- **Authentication**: Simplifies connecting to the Canvas API, prioritizing simplicity and security in credential management.
- **Course Management**: Enables access and administration of course-related information, including details on enrollments and content.
- **Participant Interactions**: Provides resources for managing and communicating with students and instructors.
- **Assignments and Submissions**: Assists in managing tasks and submissions, aiding in effective academic tracking.

The purpose of this documentation is to provide detailed information on installation, usage, and examples of `canvas_lms_connector` application. It is intended to be an informative resource for efficiently utilizing the Canvas API across various development contexts.

## Documentation Conventions

This section outlines the conventions used throughout this documentation for the Canvas LMS Connector. Understanding these conventions will help in effectively utilizing this guide and interpreting the information as intended.

### Notation and Syntax

- **Code and Commands**: Text that represents code, commands, or file names is presented in a `monospaced font`.
- **Placeholders**: Text in `<angle brackets>` indicates a placeholder that should be replaced with the relevant value by the user.
- **Environment Variables**: Names of environment variables are written in ALL_CAPS.

### Terminology

- **API**: Refers to the Application Programming Interface provided by the Canvas LMS.
- **Canvas**: The Canvas Learning Management System, the primary system with which this library interacts.

### Highlighting

- **Important Notes**: Sections marked with **Important**: contain crucial information for the operation or understanding of certain features.
- **Tips and Recommendations**: Suggestions and best practices are indicated with **Tip**: to provide additional guidance.

### Examples

- Practical examples are provided throughout the documentation to illustrate usage scenarios and code snippets.
- Example commands or code snippets can be directly copied and used, but may need to be adjusted for specific contexts or environments.

By adhering to these conventions, this documentation aims to provide a clear and consistent guide for users of the `canvas_lms_connector`. Should there be any ambiguity or questions regarding the conventions, users are encouraged to seek clarification through the support channels provided.

## Getting Started

### Prerequisites

Before beginning to work with the "Canvas LMS Connector", ensure that the following prerequisites are met:

#### System Requirements
- **Rust Environment**: A working Rust environment is necessary, as the "Canvas LMS Connector" is written in Rust. [Rustup](https://rustup.rs/) is recommended for installing the Rust toolchain.
- **Operating System**: Compatible with any standard operating system supporting Rust, including Windows, macOS, and Linux.

#### Knowledge Prerequisites
- **Basic Rust Knowledge**: Fundamental understanding of Rust programming is required for effectively using the "Canvas LMS Connector".
- **Familiarity with Canvas LMS**: Basic knowledge of Canvas LMS, including its features and API capabilities, is beneficial. For more information on the Canvas LMS API, refer to the [Canvas LMS REST API Documentation](https://canvas.instructure.com/doc/api/index.html).

#### Canvas API Access
- **Canvas Account**: Access to a Canvas LMS instance is required, which can be through a school, university, or a personal developer account.
- **API Token**: An API token from Canvas LMS is needed for authentication. This can be obtained from the Canvas LMS account settings.

Meeting these prerequisites will facilitate a smooth initial setup and an effective use of the "Canvas LMS Connector".

### Installation

To install the "Canvas LMS Connector" in your Rust project, follow these steps:

1. **Add the Dependency**:
    - Open your project's `Cargo.toml` file and add `canvas_lms_connector` to the `[dependencies]` section:
      ```toml
      [dependencies]
      canvas_lms_connector = "latest_version"
      ```

2. **Update Your Project**:
    - Run this command in your project directory to download and install the library:
      ```shell
      cargo update
      ```

3. **Build the Project**:
    - Compile your project to verify the installation:
      ```shell
      cargo build
      ```
### Initial Configuration

To configure the "Canvas LMS Connector" for first-time use, follow these steps:

1. **Obtain the Canvas API URL**:
    - Access your Canvas LMS instance. The URL is typically in the format `https://[your-institution].instructure.com`.
    - Append `/api/v1` to the end of this URL. For example, `https://[your-institution].instructure.com/api/v1`.
    - This is your Canvas API base URL.

2. **Generate an API Token**:
    - Log in to your Canvas account.
    - Navigate to `Account` > `Settings`.
    - Scroll down to `Approved Integrations` and click on `+ New Access Token`.
    - Provide a purpose for the token and set an expiration date if desired.
    - Click `Generate Token` to create a new API token.
    - Securely store the generated token, as it will not be displayed again.

After obtaining the API URL and token, you can test these credentials using the `test_canvas_credentials` function provided by the `CanvasCredentials` struct. This function helps verify the validity of the API URL and token.

Example usage:
```rust
use canvas_lms_connector::credentials::{CanvasCredentials, test_canvas_credentials};

let api_url = "https://your-institution.instructure.com/api/v1";
let access_token = "your_api_token";
let test_result = test_canvas_credentials(api_url, access_token);

match test_result {
    Ok(status_code) => println!("Credentials are valid! Status code: {}", status_code),
    Err(error_code) => eprintln!("Failed to validate credentials. Error code: {}", error_code),
}
```
With the API URL and token, you can now set up the "Canvas LMS Connector" in your project. Typically, these values are set as environment variables or configured in a settings file for security and ease of management.

### Creating Authentication Credentials Structure

The authentication process in the "Canvas LMS Connector" begins with obtaining a `CanvasCredentials` struct, which contains the necessary credentials acquired in the previous steps. This struct plays a crucial role in establishing a secure connection with the Canvas LMS API. The following subsections detail the methods available for obtaining and securely storing these credentials.

1. **Interactive Authentication via CLI (Preferred Method)**:
   - The `credentials` method first attempts to retrieve credentials stored in the system's key store.
   - Example code to retrieve credentials:
     ```rust
     use canvas_lms_connector::credentials::{CanvasCredentials, credentials};

     let credentials = credentials();
     ```
   - If not present, it will prompt the user to enter them via the console. These are then stored securely for future use.
       - **Note on System Key Store Access:**
         When using the `credentials` method to access credentials from the system's key store, the system may prompt for the user's password. This is a standard security measure to ensure authorized access to sensitive information. It's important to be prepared for this prompt, especially when running the application for the first time or on a new device.
       - **Optionally**, for file-based credentials, enable the `use_file_credentials` feature in `Cargo.toml`:
       ```toml
       [features]
       use_file_credentials = []
       ```
     - With this feature enabled, `credentials()` checks for a file `config.json` in the 'Downloads' directory that contains the following:
       ```json
       {
           "url_canvas": "https://your-institution.instructure.com/api/v1",
           "token_canvas": "your_api_token"
       }
       ```
     - If present, the credentials are retrieved from the file and used for authentication.
       - **Warning about File-Based Credentials Method:**
       While the file-based authentication method (`use_file_credentials`) is convenient, particularly for development purposes, it poses significant security risks and is not recommended for production environments. Storing credentials in a file, such as `config.json`, can make them vulnerable to unauthorized access. This method should be used with extreme caution and only for development and testing purposes, ensuring that production credentials are managed in a more secure manner.

2. **Using the `CanvasCredentials` Structure (For Development Purposes Only)**:
   - Direct initialization is recommended only for development.
   - Example code:
     ```rust
     use canvas_lms_connector::credentials::CanvasCredentials;

     let credentials = CanvasCredentials {
         url_canvas: "https://your-institution.instructure.com/api/v1".to_string(),
         token_canvas: "your_api_token".to_string(),
     };
     ```

These methods ensure authentication with the Canvas LMS for API interactions.

### Retrieving Courses

Retrieving courses from the Canvas LMS using the "Canvas LMS Connector" involves two methods returning distinct result types: `CanvasResultCourses` for multiple courses and `CanvasResultSingleCourse` for a single course.

**`CanvasResultCourses` Structure**:
- Variants:
    - `Ok(Vec<Course>)`: Success with a list of courses.
    - `ErrConnection(String)`: Error related to connection issues.
    - `ErrCredentials(String)`: Error related to authentication or credentials.

**`CanvasResultSingleCourse` Structure**:
- Variants:
    - `Ok(Course)`: Success with a single course.
    - `ErrConnection(String)`: Error related to connection issues.
    - `ErrCredentials(String)`: Error related to authentication or credentials.

#### Fetching All Courses:
```rust
use canvas_lms_connector::{Canvas, CanvasCredentials, CanvasResultCourses};

let credentials = CanvasCredentials { ... }; // Initialize with your credentials
match Canvas::fetch_courses_with_credentials(&credentials) {
    CanvasResultCourses::Ok(courses) => println!("Courses: {:?}", courses),
    CanvasResultCourses::ErrConnection(err) => eprintln!("Connection error: {}", err),
    CanvasResultCourses::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
}
```
#### Fetching a Single Course by ID:
```rust
use canvas_lms_connector::{Canvas, CanvasCredentials, CanvasResultSingleCourse};

let credentials = CanvasCredentials { ... }; // Initialize with your credentials
let course_id: u64 = 123; // Your course ID
match Canvas::fetch_single_course_with_credentials(&credentials, course_id) {
CanvasResultSingleCourse::Ok(course) => println!("Course: {:?}", course),
CanvasResultSingleCourse::ErrConnection(err) => eprintln!("Connection error: {}", err),
CanvasResultSingleCourse::ErrCredentials(err) => eprintln!("Credentials error: {}", err),
}
```



++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

The "Canvas LMS Connector" is now installed and ready for use in your Rust project.

## Documentation

For detailed documentation on how to use Canvas Learning, refer to the [official documentation](https://crates.io/crates/canvas_lms_connector).


## License

This project is licensed under the MIT License
