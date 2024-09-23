use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Rubric {
    pub context_id: u64,
    pub context_type: String,
    pub data: Vec<Criterion>,
    pub points_possible: f64,
    pub id: u64,
    pub title: String,  // This should match the JSON field "title"
    pub free_form_criterion_comments: Option<bool>,  // Optional field based on your JSON
    pub hide_score_total: Option<bool>,              // Optional field
    pub public: Option<bool>,                        // Optional field
    pub rating_order: Option<String>,                // Optional field
    pub read_only: Option<bool>,                     // Optional field
    pub reusable: Option<bool>,                      // Optional field
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Criterion {
    pub criterion_use_range: Option<bool>,
    pub description: String,
    pub id: String,
    pub long_description: Option<String>,
    pub points: f64,
    pub ratings: Vec<Rating>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rating {
    pub criterion_id: String,
    pub description: String,
    pub id: String,
    pub long_description: String,
    pub points: f64,
}
