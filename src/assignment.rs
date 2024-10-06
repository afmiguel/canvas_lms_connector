// Import necessary crates and modules
use crate::rubric_downloaded::RubricDownloaded;
use crate::submission::Submission;
use crate::{canvas, CourseInfo, Student};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::sync::Arc;

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
    pub due_at: Option<DateTime<Utc>>, // Campo opcional para a data de vencimento
    pub rubric_id: Option<u64>,
    pub group_category_id: Option<u64>,
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
    pub fn fetch_submissions(
        &self,
        students: &Vec<Student>,
    ) -> Result<Vec<Submission>, Box<dyn std::error::Error>> {
        let groups = match canvas::fetch_groups_for_assignment(
            self.info.as_ref(),
            self.info.course_info.canvas_info.as_ref(),
        ) {
            Ok(groups) => {
                if groups.is_empty() {
                    None
                } else {
                    Some(groups)
                }
            }
            Err(_) => None,
        };

        match canvas::get_all_submissions(
            self.info.course_info.canvas_info.as_ref(),
            self.info.course_info.id,
            self.info.id,
            groups.is_some(),
        ) {
            Ok(submissions_value) => {
                // Recupera todos os estudantes do curso
                let all_course_students = self.info.course_info.fetch_students()?;

                let submissions = submissions_value
                    .iter()
                    .filter_map(|j| {
                        Submission::convert_json_to_submission(
                            &all_course_students,
                            j,
                            &self.info.clone(),
                            &groups,
                        )
                    })
                    .collect::<Vec<_>>();

                // Elimina as submissões que não são relacionadas aos estudantes de students
                let submissions = submissions
                    .into_iter()
                    .filter(|submission| {
                        submission
                            .students_info
                            .iter()
                            .any(|si| students.iter().any(|student| student.info.id == si.id))
                    })
                    .collect::<Vec<_>>();

                // println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
                // println!(
                //     "Submissões ({}/{}):\n{:#?}",
                //     submissions.len(),
                //     submissions_value.len(),
                //     submissions
                // );
                // println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");

                Ok(submissions)
            }
            Err(e) => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to fetch submissions with error: {}", e),
            ))),
        }
    }

    pub fn download_rubric(&self) -> Option<RubricDownloaded> {
        if let Some(rubric_id) = self.info.rubric_id {
            match canvas::download_rubric(
                self.info.course_info.canvas_info.as_ref(),
                self.info.course_info.id,
                rubric_id,
            ) {
                Ok(rubric_value) => {
                    // Imprime o valor da rubrica
                    // println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
                    // println!("Rubrica: {:?}", rubric_value);
                    // println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
                    // Adiciona `assignment_info` ao deserializar o JSON para o struct Rubric
                    let rubric_result: Result<RubricDownloaded, _> =
                        serde_json::from_value(rubric_value);

                    match rubric_result {
                        Ok(rubric) => {
                            // Inicializar o campo `assignment_info` com a referência ao assignment atual
                            // rubric.assigment_info = Arc::clone(&self.info);
                            Some(rubric) // Sucesso ao deserializar e inicializar
                        }
                        Err(e) => {
                            eprintln!("Erro ao deserializar rubrica: {}", e);
                            None // Falha ao deserializar a rubrica
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Erro ao baixar rubrica: {}", e);
                    None // Falha ao baixar a rubrica
                }
            }
        } else {
            eprintln!("Rubric ID não encontrado para este assignment.");
            None // Rubric ID não encontrado
        }
    }

    /// Retrieves a specific submission for this assignment based on the submission ID.
    ///
    /// This method makes an API call to fetch the details of a particular submission for this assignment
    /// by the provided submission ID. It returns the `Submission` object containing the submission's
    /// details or an error if the submission is not found or if the request fails.
    ///
    /// # Parameters
    ///
    /// - `submission_id`: The unique identifier of the submission in the Canvas LMS.
    ///
    /// # Returns
    ///
    /// Returns a `Result<Submission, Box<dyn Error>>`, where:
    /// - `Ok(Submission)` contains the successfully loaded submission data.
    /// - `Err(Box<dyn Error>)` contains an error message in case the submission is not found or any issue occurs.
    ///
    /// # Example
    ///
    /// ```
    /// let assignment = Assignment { /* initialized */ };
    /// let submission_id = 12345;
    /// match assignment.get_submission_from_submission_id(submission_id) {
    ///     Ok(submission) => println!("Submission loaded successfully: {:?}", submission),
    ///     Err(e) => eprintln!("Error loading submission: {}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns errors if the submission is not found or if there is a failure in the API request.
    pub fn get_submission_from_submission_id(
        &self,
        submission_id: u64,
        mut cache: Option<&mut GetSubmissionFromSubmissionIdCache>,
    ) -> Result<Submission, Box<dyn Error>> {
        // Fetch all submissions for the assignment

        // Variáveis para submissões e estudantes
        let submissions_value: Vec<Value>;
        let all_students_value: Vec<Student>;

        // Primeiro: lidar com o cache de submissões
        if let Some(ref mut cache) = cache {
            if let Some(submissions) = cache.submissions_value.as_ref() {
                submissions_value = submissions.clone(); // Usa submissões do cache
            } else {
                submissions_value = canvas::get_all_submissions(
                    self.info.course_info.canvas_info.as_ref(),
                    self.info.course_info.id,
                    self.info.id,
                    self.info.group_category_id.is_some(),
                )?; // Faz requisição se não houver cache
                cache.submissions_value = Some(submissions_value.clone()); // Atualiza o cache
            }
        } else {
            submissions_value = canvas::get_all_submissions(
                self.info.course_info.canvas_info.as_ref(),
                self.info.course_info.id,
                self.info.id,
                self.info.group_category_id.is_some(),
            )?; // Faz requisição se o cache não for fornecido
        }

        // Segundo: lidar com o cache de estudantes
        if let Some(ref mut cache) = cache {
            if let Some(students) = cache.submission.as_ref() {
                all_students_value = students.clone(); // Usa estudantes do cache
            } else {
                all_students_value = self.info.course_info.fetch_students()?; // Faz requisição se não houver cache
                cache.submission = Some(all_students_value.clone()); // Atualiza o cache
            }
        } else {
            all_students_value = self.info.course_info.fetch_students()?; // Faz requisição se o cache não for fornecido
        }

        let groups = match canvas::fetch_groups_for_assignment(
            self.info.as_ref(),
            self.info.course_info.canvas_info.as_ref(),
        ) {
            Ok(groups) => {
                if groups.is_empty() {
                    None
                } else {
                    Some(groups)
                }
            }
            Err(_) => None,
        };

        // Tentar encontrar a submissão com o ID fornecido
        match submissions_value
            .iter()
            .filter_map(|j| {
                Submission::convert_json_to_submission(
                    &all_students_value,
                    j,
                    &self.info.clone(),
                    &groups,
                )
            })
            .find(|submission| submission.id == submission_id)
        {
            Some(submission) => Ok(submission),
            None => Err(format!("Submission with id {} not found", submission_id).into()),
        }
    }

    /// Deleta um comentário de uma submissão associada a esta tarefa.
    ///
    /// Este método chama a função `delete_comment` definida em `canvas.rs` para
    /// realizar a operação de deletar o comentário.
    ///
    /// # Parâmetros
    /// - `client`: O cliente HTTP para realizar a requisição.
    /// - `student_id`: O ID do estudante cuja submissão contém o comentário a ser deletado.
    /// - `comment_id`: O ID do comentário que será deletado.
    ///
    /// # Retorno
    /// Retorna `Ok(())` em caso de sucesso ou um `Err(Box<dyn Error>)` em caso de falha.
    pub fn delete_comment(
        &self,
        student_id: u64,
        comment_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Chama a função delete_comment já implementada em canvas.rs
        canvas::delete_comment(
            &self.info.course_info.canvas_info, // Credenciais do Canvas
            self.info.course_info.id,           // ID do curso
            self.info.id,                       // ID da tarefa (assignment_id)
            student_id,                         // ID do estudante
            comment_id,                         // ID do comentário a ser deletado
        )
    }
}
pub struct GetSubmissionFromSubmissionIdCache {
    pub submissions_value: Option<Vec<Value>>,
    pub submission: Option<Vec<Student>>,
}

impl GetSubmissionFromSubmissionIdCache {
    pub fn new() -> Self {
        Self {
            submissions_value: None,
            submission: None,
        }
    }
}
