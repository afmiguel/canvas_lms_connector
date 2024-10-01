use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
/// Structure representing a rubric to be sent to Canvas LMS. This contains the rubric's
/// details and its association with a course or other entities within Canvas.
pub struct CanvasRubricSubmission {
    pub rubric: RubricSubmissionDetails,  // Details about the rubric to be sent
    pub rubric_association: RubricAssociationSubmission,  // Information about where the rubric is associated (e.g., a course)
}

#[derive(Debug, Serialize, Deserialize)]
/// Details of the rubric to be sent, including title and criteria.
/// Criteria are stored in a map with numerical string keys.
pub struct RubricSubmissionDetails {
    pub title: String,  // Title of the rubric
    pub criteria: HashMap<String, CriterionSubmission>,  // Criteria map, indexed by numerical string keys (e.g., "1", "2")
}

#[derive(Debug, Serialize, Deserialize)]
/// Structure representing a single criterion in the rubric.
/// Contains a description, whether it uses a point range, and the ratings (levels of achievement).
pub struct CriterionSubmission {
    pub description: String,  // Description of the criterion
    pub criterion_use_range: Option<bool>,  // Indicates if this criterion uses a range of points
    pub ratings: HashMap<String, RatingSubmission>,  // Ratings map, indexed by numerical string keys (e.g., "1", "2")
}

#[derive(Debug, Serialize, Deserialize)]
/// Structure representing a rating (level of achievement) for a criterion.
/// Contains a description of the rating and the points associated with it.
pub struct RatingSubmission {
    pub description: String,  // Description of the rating level (e.g., "Excellent", "Good", etc.)
    pub points: f64,  // Points assigned for this rating
}

#[derive(Debug, Serialize, Deserialize)]
/// Structure representing the association of the rubric to a specific entity in Canvas (e.g., a course).
/// This includes the type of association, the ID of the entity, and whether it is used for grading.
pub struct RubricAssociationSubmission {
    pub association_type: String,  // Type of association, such as "Course"
    pub association_id: u64,  // ID of the associated entity (e.g., course ID)
    pub use_for_grading: bool,  // Indicates whether the rubric is used for grading
}

impl CanvasRubricSubmission {
    /// Creates a new CanvasRubricSubmission with the provided rubric details and association.
    pub fn new(rubric: RubricSubmissionDetails, rubric_association: RubricAssociationSubmission) -> Self {
        CanvasRubricSubmission {
            rubric,
            rubric_association,
        }
    }
}

use serde_json::from_reader;
use std::fs::File;
use std::io::BufReader;
use std::error::Error;

impl CanvasRubricSubmission {
    /// Loads a `CanvasRubricSubmission` from a JSON file.
    pub fn load_from_json(file_path: &str) -> Result<Self, Box<dyn Error>> {
        // Open the file
        let file = File::open(file_path)?;

        // Create a buffered reader for the file
        let reader = BufReader::new(file);

        // Deserialize the JSON content into CanvasRubricSubmission
        let rubric: CanvasRubricSubmission = from_reader(reader)?;

        // Return the deserialized rubric
        Ok(rubric)
    }
}
