use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub type MeasuresData = HashMap<String, MeasureSet>;

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureSet {
    pub measures: Vec<(i64, f64)>,
    pub unit: String,
    pub min: f64,
    pub max: f64,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measure {
    pub target: String,
    pub time: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    pub process_id: String,
    pub stream_id: String,
}
