// Import necessary crates and modules
use std::sync::Arc;
use chrono::{DateTime, Utc};
// use std::thread::sleep;
use serde::{Deserialize, Serialize};
use crate::{canvas, CourseInfo, Student};
use crate::submission::{Submission, SubmissionType};
use serde_json::Value;
use crate::rubric::Rubric;
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
    pub rubric_id: Option<u64>,
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
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to fetch submissions with error: {}", e),
                )))
            }
        }
    }

    fn convert_json_to_submission(
        students: &Vec<Student>,
        j: &Value,
    ) -> Option<Submission> {
        for student in students {
            if let Some(user_id) = j["user_id"].as_u64() {
                if student.info.id == user_id {
                    let file_ids = j["attachments"]
                        .as_array()
                        .map_or(Vec::new(), |attachments| {
                            attachments
                                .iter()
                                .filter_map(|attachment| attachment["id"].as_u64())
                                .collect()
                        });

                    return Some(Submission {
                        id: j["id"].as_u64().unwrap(),
                        assignment_id: j["assignment_id"].as_u64().unwrap(),
                        score: j["score"].as_f64(),
                        submitted_at: j["submitted_at"].as_str().map(|s| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)),
                        submission_type: j["submission_type"]
                            .as_str()
                            .map(|st| match st {
                                "online_upload" => SubmissionType::OnlineUpload,
                                "online_text_entry" => SubmissionType::OnlineTextEntry,
                                "online_url" => SubmissionType::OnlineUrl,
                                "media_recording" => SubmissionType::MediaRecording,
                                "none" => SubmissionType::None,
                                _ => SubmissionType::Other, // Corrigido para a variante unitária
                            }),
                        student: student.info.clone(),
                        file_ids,
                    });
                }
            }
        }
        None
    }




    pub fn download_rubric(
        &self,
    ) -> Option<Rubric> {
        let client = &reqwest::blocking::Client::new();

        if let Some(rubric_id) = self.info.rubric_id {
            match canvas::download_rubric(&client, self.info.course_info.canvas_info.as_ref(), self.info.course_info.id, rubric_id) {
                Ok(rubric_value) => {
                    // Adiciona `assignment_info` ao deserializar o JSON para o struct Rubric
                    let rubric_result: Result<Rubric, _> = serde_json::from_value(rubric_value);

                    match rubric_result {
                        Ok(rubric) => {
                            // Inicializar o campo `assignment_info` com a referência ao assignment atual
                            // rubric.assigment_info = Arc::clone(&self.info);
                            Some(rubric)  // Sucesso ao deserializar e inicializar
                        }
                        Err(e) => {
                            eprintln!("Erro ao deserializar rubrica: {}", e);
                            None  // Falha ao deserializar a rubrica
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Erro ao baixar rubrica: {}", e);
                    None  // Falha ao baixar a rubrica
                }
            }
        } else {
            eprintln!("Rubric ID não encontrado para este assignment.");
            None  // Rubric ID não encontrado
        }
    }
}

