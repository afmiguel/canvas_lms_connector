use std::collections::HashMap;
use std::error::Error;
// Import necessary crates and modules
use crate::{canvas, AssignmentInfo, Course, Student, StudentInfo};
use chrono::{DateTime, Duration, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Comment {
    pub id: u64,         // ID do comentário
    pub content: String, // Conteúdo do comentário
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Submission {
    pub id: u64,                                 // Submission's unique identifier
    pub assignment_id: u64,                      // Assignment's unique identifier
    pub score: Option<f64>,                      // Graded score, optional
    pub submitted_at: Option<DateTime<Utc>>,     // Submission timestamp, optional
    pub submission_type: Option<SubmissionType>, // Tipo de submissão, agora tratado como Option
    // #[serde(skip)]
    // pub student_info: Arc<StudentInfo>,
    #[serde(skip)]
    pub students_info: Vec<Arc<StudentInfo>>,
    #[serde(skip)]
    pub assignment_info: Arc<AssignmentInfo>,
    #[serde(skip)]
    pub file_ids: Vec<u64>, // IDs dos arquivos associados
    pub comments: Vec<Comment>, // Lista de comentários, agora incluindo o ID do comentário
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
        // Pega o primeiro estudante da lista
        let student_info = match self.students_info.first() {
            Some(student_info) => student_info,
            None => return Err("No student info found".into()),
        };

        let course = Course {
            info: student_info.course_info.clone(),
        };
        course.comment_with_file(
            client,
            self.assignment_id,
            student_info.id,
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
    /// let course = Course { /* ... */ };
    /// match course.update_assignment_score(assignment_id, student_id, new_score) {
    ///     Ok(_) => /* handle success */,
    ///     Err(e) => /* handle error */,
    /// }
    /// ```
    pub fn update_score(&mut self, new_score: Option<f64>) -> Result<(), Box<dyn Error>> {
        // Pega o primeiro estudante da lista
        let student_info = match self.students_info.first() {
            Some(student_info) => student_info,
            None => return Err("No student info found".into()),
        };

        let course = Course {
            info: student_info.course_info.clone(),
        };

        let ret = course.update_assignment_score(self.assignment_id, student_info.id, new_score);
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
        output_dir: &str,
    ) -> Result<Option<Vec<String>>, Box<dyn std::error::Error>> {
        // Cria o diretório de saída, se não existir
        std::fs::create_dir_all(output_dir)?;

        // Vetor para armazenar os caminhos completos dos arquivos baixados
        let mut downloaded_files = Vec::new();

        // Pega o primeiro estudante da lista
        let student_info = match self.students_info.first() {
            Some(student_info) => student_info,
            None => return Err("No student info found".into()),
        };

        // Itera sobre os IDs dos arquivos e faz o download de cada um
        for &file_id in &self.file_ids {
            // Faz o download do arquivo e obtém o caminho completo onde foi salvo
            let file_path = canvas::download_file(
                &student_info.course_info.canvas_info.client,
                &student_info.course_info.canvas_info, // Passa as credenciais do Canvas
                file_id,
                output_dir, // Caminho onde o arquivo será salvo
            )?;

            // Adiciona o caminho completo do arquivo baixado à lista
            downloaded_files.push(file_path);
        }

        //        println!("All files downloaded for submission {}", self.id);

        // Retorna a lista de caminhos completos dos arquivos baixados
        if downloaded_files.is_empty() {
            Ok(None)
        } else {
            Ok(Some(downloaded_files))
        }
    }

    // Deleta um comentário associado a esta submissão.
    ///
    /// Este método chama a função `delete_comment` definida em `canvas.rs` para
    /// realizar a operação de deletar o comentário.
    ///
    /// # Parâmetros
    /// - `client`: O cliente HTTP para realizar a requisição.
    /// - `comment_id`: O ID do comentário que será deletado.
    ///
    /// # Retorno
    /// Retorna `Ok(())` em caso de sucesso ou um `Err(Box<dyn Error>)` em caso de falha.
    pub fn delete_comment(&self, comment_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        // Pega o primeiro estudante da lista
        let student_info = match self.students_info.first() {
            Some(student_info) => student_info,
            None => return Err("No student info found".into()),
        };

        // Chama a função delete_comment já implementada em canvas.rs
        canvas::delete_comment(
            &self.assignment_info.course_info.canvas_info, // Credenciais do Canvas
            self.assignment_info.course_info.id,           // ID do curso
            self.assignment_id,                            // ID da tarefa (assignment_id)
            student_info.id,                               // ID do estudante
            comment_id,                                    // ID do comentário a ser deletado
        )
    }

    /// Função que converte o JSON de submissões em uma estrutura `Submission`.
    pub(crate) fn convert_json_to_submission(
        all_course_students: &Vec<Student>,
        j: &Value,
        assignment_info: &Arc<AssignmentInfo>,
        groups: &Option<HashMap<u64, Vec<u64>>>,
    ) -> Option<Submission> {
        for student in all_course_students {
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

                    // Processa os comentários da submissão
                    let comments =
                        j["submission_comments"]
                            .as_array()
                            .map_or(Vec::new(), |comments_array| {
                                comments_array
                                    .iter()
                                    .filter_map(|comment| {
                                        // Captura o ID e o conteúdo do comentário
                                        let id = comment["id"].as_u64();
                                        let content = comment["comment"].as_str().map(String::from);

                                        // Se ambos o ID e o conteúdo do comentário existirem
                                        if let (Some(id), Some(content)) = (id, content) {
                                            Some(Comment { id, content })
                                        } else {
                                            None
                                        }
                                    })
                                    .collect()
                            });

                    // Localiza o grupo do estudante
                    let group_id = groups.as_ref().and_then(|groups| {
                        groups
                            .iter()
                            .find(|(_, students)| students.contains(&student.info.id))
                    });

                    // Se achou um group_id, cria um vetor Vec<StudentInfo> com os estudantes do grupo
                    let mut students_info = group_id.map_or(Vec::new(), |(_, student_ids)| {
                        student_ids
                            .iter()
                            .filter_map(|student_id| {
                                all_course_students
                                    .iter()
                                    .find(|student| student.info.id == *student_id)
                                    .map(|student| student.info.clone())
                            })
                            .collect()
                    });

                    if students_info.is_empty() {
                        // Se está vazio significa que não é por grupo. Inclui o estudante com user_id
                        if let Some(student) = all_course_students
                            .iter()
                            .find(|student| student.info.id == user_id)
                        {
                            students_info.push(student.info.clone());
                        } else {
                            panic!("Falha ao associar um estudante a uma submissão.");
                        }
                    }

                    return Some(Submission {
                        id: j["id"].as_u64().unwrap(),
                        assignment_id: j["assignment_id"].as_u64().unwrap(),
                        score: j["score"].as_f64(),
                        submitted_at: j["submitted_at"]
                            .as_str()
                            .map(|s| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)),
                        submission_type: j["submission_type"].as_str().map(|st| match st {
                            "online_upload" => SubmissionType::OnlineUpload,
                            "online_text_entry" => SubmissionType::OnlineTextEntry,
                            "online_url" => SubmissionType::OnlineUrl,
                            "media_recording" => SubmissionType::MediaRecording,
                            "none" => SubmissionType::None,
                            _ => SubmissionType::Other,
                        }),
                        // student_info: student.info.clone(),
                        students_info,
                        file_ids,
                        assignment_info: assignment_info.clone(),
                        comments,
                    });
                }
            }
        }
        None
    }
}
