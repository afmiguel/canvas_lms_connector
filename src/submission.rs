use std::error::Error;
// Import necessary crates and modules
use crate::{canvas, AssignmentInfo, Course, StudentInfo};
use chrono::{DateTime, Duration, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionType {
    OnlineUpload,
    OnlineTextEntry,
    OnlineUrl,
    MediaRecording,
    None,
    #[serde(other)] // Tratamento para tipos desconhecidos
    Other,
}

impl Default for SubmissionType {
    fn default() -> Self {
        SubmissionType::None
    }
}

impl SubmissionType {
    pub fn as_str(&self) -> &str {
        match self {
            SubmissionType::OnlineUpload => "online_upload",
            SubmissionType::OnlineTextEntry => "online_text_entry",
            SubmissionType::OnlineUrl => "online_url",
            SubmissionType::MediaRecording => "media_recording",
            SubmissionType::None => "none",
            SubmissionType::Other => "other",
        }
    }
}

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
/// Enum representing the type of submission.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Submission {
    pub id: u64,                                 // Submission's unique identifier
    pub assignment_id: u64,                      // Assignment's unique identifier
    pub score: Option<f64>,                      // Graded score, optional
    pub submitted_at: Option<DateTime<Utc>>,     // Submission timestamp, optional
    pub submission_type: Option<SubmissionType>, // Tipo de submissão, agora tratado como Option
    #[serde(skip)]
    pub student_info: Arc<StudentInfo>,
    #[serde(skip)]
    pub assignment_info: Arc<AssignmentInfo>,
    #[serde(skip)]
    pub file_ids: Vec<u64>, // IDs dos arquivos associados
}

impl Submission {
    /// Checks if the submission is late by comparing `submitted_at` with `due_at`.
    ///
    /// Returns:
    /// - `Some(Duration)` if the submission is late, indicating the time difference between `submitted_at` and `due_at`.
    /// - `None` if the submission is not late or if there is no submission or due date.
    pub fn is_late(&self) -> Option<Duration> {
        // Check if both submission and due dates are available
        if let (Some(submitted_at), Some(due_at)) = (self.submitted_at, self.assignment_info.due_at)
        {
            // Compare the dates
            if submitted_at > due_at {
                // Calculate the time difference between submission and due date
                Some(submitted_at.signed_duration_since(due_at))
            } else {
                None // Submission is not late
            }
        } else {
            None // Missing information to determine lateness
        }
    }

    /// Formats the late submission duration as a human-readable string.
    ///
    /// Calls the `is_late` function to check if the submission is late, and if so,
    /// formats the resulting `Duration` into a string in the form of "Xh Ym Zs".
    ///
    /// Returns:
    /// - `Some(String)` if the submission is late, indicating how late the submission was.
    /// - `None` if the submission is not late or if there is no submission or due date.
    pub fn is_late_str(&self) -> Option<String> {
        // Call the is_late function to get the late Duration
        if let Some(late_duration) = self.is_late() {
            let secs = late_duration.num_seconds().abs();
            let hours = secs / 3600;
            let minutes = (secs % 3600) / 60;
            let seconds = secs % 60;

            // Format the duration in a human-readable way
            let formatted_duration = if hours > 0 {
                format!("{}h {:02}m {:02}s", hours, minutes, seconds)
            } else if minutes > 0 {
                format!("{}m {:02}s", minutes, seconds)
            } else {
                format!("{}s", seconds)
            };

            Some(formatted_duration)
        } else {
            None // Not late or missing information
        }
    }

    /// Adds a file comment to a student's assignment submission.
    ///
    /// This function first uploads a file to the Canvas LMS and then attaches it as a comment
    /// to a specific assignment submission by a student. It also adds text content to the comment.
    ///
    /// Arguments:
    /// - `client`: HTTP client for executing requests.
    /// - `file_path`: Optional path to the file to be uploaded.
    /// - `comment_text`: Text content of the comment.
    ///
    /// Returns:
    /// - `Result<(), Box<dyn Error>>`: Success or an error detailing any issues encountered.
    ///
    /// Example:
    /// ```
    /// let client = reqwest::blocking::Client::new();
    /// let course = Course { /* ... */ };
    /// match course.add_file_comment(&client, assignment_id, student_id, Some("path/to/file.pdf"), "Great work!") {
    ///     Ok(_) => /* handle success */,
    ///     Err(e) => /* handle error */,
    /// }
    /// ```
    pub fn comment_with_file(
        &self,
        client: &Client,
        file_path: Option<&str>,
        comment_text: &str,
    ) -> Result<(), Box<dyn Error>> {
        let course = Course {
            info: self.student_info.course_info.clone(),
        };
        course.comment_with_file(
            client,
            self.assignment_id,
            self.student_info.id,
            file_path,
            comment_text,
        )
    }

    /// Updates the score of a student's assignment submission.
    ///
    /// Sends an HTTP PUT request to the Canvas API to update the score for a specific assignment
    /// submission by a student. Handles request construction, execution, and authentication using
    /// Canvas API credentials.
    ///
    /// Arguments:
    /// - `client`: HTTP client for executing requests.
    /// - `new_score`: New score to be set, or `None` to clear the existing score.
    ///
    /// Returns:
    /// - `Result<(), Box<dyn std::error::Error>>`: Success or an error detailing any issues encountered.
    ///
    /// Example:
    /// ```
    /// let client = reqwest::blocking::Client::new();
    /// let course = Course { /* ... */ };
    /// match course.update_assignment_score(&client, assignment_id, student_id, new_score) {
    ///     Ok(_) => /* handle success */,
    ///     Err(e) => /* handle error */,
    /// }
    /// ```
    pub fn update_score(
        &mut self,
        client: &Client,
        new_score: Option<f64>,
    ) -> Result<(), Box<dyn Error>> {
        let course = Course {
            info: self.student_info.course_info.clone(),
        };

        let ret =
            course.update_assignment_score(client, self.assignment_id, self.student_info.id, new_score);
        self.score = new_score;
        ret
    }

    /// Downloads all files associated with this submission.
    ///
    /// This method iterates over the `file_ids` associated with the submission and
    /// downloads each file to the specified output path. It uses the `download_submission_file`
    /// function for each file.
    ///
    /// Arguments:
    /// - `client`: HTTP client for executing requests.
    /// - `output_path`: Path where the files will be saved. Each file will be saved with its original name.
    ///
    /// Returns:
    /// - `Result<(), Box<dyn Error>>`: Success or an error detailing any issues encountered.
    ///
    /// Example:
    /// ```
    /// let client = reqwest::blocking::Client::new();
    /// submission.download_submission_files(&client, "output/directory")?;
    /// ```
    pub fn download_submission_files(
        &self,
        client: &Client,
        output_dir: &str,
    ) -> Result<Option<Vec<String>>, Box<dyn std::error::Error>> {
        // Cria o diretório de saída, se não existir
        std::fs::create_dir_all(output_dir)?;

        // Vetor para armazenar os caminhos completos dos arquivos baixados
        let mut downloaded_files = Vec::new();

        // Itera sobre os IDs dos arquivos e faz o download de cada um
        for &file_id in &self.file_ids {
            // Faz o download do arquivo e obtém o caminho completo onde foi salvo
            let file_path = canvas::download_submission_file(
                client,
                &self.student_info.course_info.canvas_info, // Passa as credenciais do Canvas
                file_id,
                output_dir, // Caminho onde o arquivo será salvo
            )?;

            // Adiciona o caminho completo do arquivo baixado à lista
            downloaded_files.push(file_path);
        }

        //        println!("All files downloaded for submission {}", self.id);

        // Retorna a lista de caminhos completos dos arquivos baixados
        Ok(Some(downloaded_files))
    }
}
