use serde_json::from_str;
use serde::Deserialize;
use std::fs::read_to_string;


#[derive(Deserialize, Debug, Clone)]
pub struct Instance {
    resources: usize,
    horizon: usize,
    duration_regular: usize,
    duration_major: usize,
    time_regular: usize,
    tasks: Vec<Task>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Task {
    start: usize,
    length: usize
}

impl Instance {
    pub fn new_from_file(path: &str) -> Self {
        from_str(&read_to_string(path).unwrap()).unwrap()
    }

    pub fn resources(&self) -> usize {
        self.resources
    }
    
    pub fn horizon(&self) -> usize {
        self.horizon
    }
    
    pub fn duration_regular(&self) -> usize {
        self.duration_regular
    }
    
    pub fn duration_major(&self) -> usize {
        self.duration_major
    }
        
    pub fn time_regular(&self) -> usize {
        self.time_regular
    }

    pub fn tasks(&self) -> &Vec<Task> {
        &self.tasks
    }
}

impl Task {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn end(&self) -> usize {
        self.start + self.length
    }
}