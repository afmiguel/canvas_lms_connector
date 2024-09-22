use serde::{Deserialize, Serialize};
// use crate::AssignmentInfo;
// use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Rubric {
    pub context_id: u64,
    pub context_type: String,
    pub data: Vec<Criterion>,
    pub points_possible: f64,
    pub id: u64,
    pub title: String,
    // #[serde(skip)] // Skipped during serialization/deserialization
    // pub assigment_info: Arc<AssignmentInfo>,          // Shared reference to student information
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Criterion {
    pub criterion_use_range: Option<String>,
    pub description: String,
    pub id: String,
    pub ignore_for_scoring: Option<bool>,
    pub long_description: Option<String>,
    pub mastery_points: Option<f64>,
    pub points: f64,
    pub ratings: Vec<Rating>,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rating {
    pub criterion_id: String,
    pub description: String,
    pub id: String,
    pub long_description: String,
    pub points: f64,
}