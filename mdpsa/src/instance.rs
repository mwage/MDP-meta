use serde_json::from_str;
use serde::Deserialize;
use std::fs::read_to_string;


#[derive(Deserialize, Debug)]
pub struct Instance {
    resources: usize,
    horizon: usize,
    duration_regular: usize,
    duration_major: usize,
    time_regular: usize,
    tasks: Vec<Task>
}

#[derive(Deserialize, Debug)]
pub struct Task {
    start: usize,
    length: usize
}

impl Instance {
    pub fn new_from_file(path: &str) -> Self {
        from_str(&read_to_string(path).unwrap()).unwrap()
    }
}