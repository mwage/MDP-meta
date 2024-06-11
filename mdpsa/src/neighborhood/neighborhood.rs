use crate::instance::Instance;

use super::State;
use super::*;
use rand::{Rng, prelude::*};


pub struct Neighborhood {
    state: State,
    last_changes: Vec<ChangeToken>
}

impl Neighborhood {
    pub fn new(instance: Instance) -> Self {
        let mut state = State::new(instance, 1);
        state.initialize();

        Neighborhood { 
            state,
            last_changes: Vec::new()
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn get_next(&mut self) -> f64 {
        let neighborhoods: Vec<Box<dyn NeighborhoodFunction>> = vec![
            Box::new(MoveRM::new(false, 100)),
            Box::new(MoveRM::new(true, 100)),
            Box::new(MoveMM::new(false, 100)),
            Box::new(MoveMM::new(true, 100)),
            Box::new(RemoveRM::new(false)),
            Box::new(RemoveRM::new(true)),
            Box::new(MoveRMToAny::new(false)),
            Box::new(MoveRMToAny::new(true)),
            Box::new(CoverTask::new(false)),
            Box::new(CoverTask::new(true)),
            Box::new(MoveMMToAny::new(false)),
            Box::new(MoveMMToAny::new(true)),
            Box::new(AddTask::new(false, true)),
            Box::new(RemoveTask::new()),
            Box::new(AddMM::new(true)),
        ];
        let selected = neighborhoods.choose(&mut thread_rng()).unwrap();
        // println!("-------------------------------\n{}", selected.to_string());
        let (delta, tokens) = selected.get_neighbor(&mut self.state);
        self.last_changes = tokens;
        
        delta
    }

    pub fn accept(&mut self) {
        self.last_changes = Vec::new();
    }

    pub fn reject(&mut self) {
        for token in self.last_changes.iter().rev() {
            match token {
                ChangeToken::MovedRM(res, prev, new) => {
                    self.state.remove_regular_maintenance(*res, *new);
                    self.state.add_regular_maintenance(*res, *prev);
                },
                ChangeToken::AddRM(res, new_rm) => self.state.remove_regular_maintenance(*res, *new_rm),
                ChangeToken::RemoveRM(res, time) => self.state.add_regular_maintenance(*res, *time),
                ChangeToken::MovedMM(res, prev) => {
                    self.state.remove_major_maintenance(*res);
                    self.state.add_major_maintenance(*res, *prev);
                },
                ChangeToken::AddMM(res) => self.state.remove_major_maintenance(*res),
                ChangeToken::RemoveMM(res, time) => self.state.add_major_maintenance(*res, *time),
                ChangeToken::AddTask(task_id) => self.state.remove_task(*task_id),
                ChangeToken::RemoveTask(res, task_id) => self.state.add_task(*res, *task_id),
                // ChangeToken::AddRM(res, new_rm) => self.state.remove_regular_maintenance(*res, *new_rm),
            }
        }
        self.last_changes = Vec::new()
    }

    pub fn increase_penalty_multi(&mut self) {
        self.state.increase_penalty_multi();
    }
    
    fn instance(&self) -> &Instance {
        &self.state.instance()
    }

    // remove a regular maintenance 
    fn remove_rm(&mut self) -> f64 {
        unimplemented!()
    }

    // add a regular maintenance 
    fn cover_task(&mut self) -> f64 {
        unimplemented!()
    }

    // Move a major maintenance within boundries between neighbors and other MMs
    fn move_mm(&mut self) -> f64 {
        unimplemented!()
    }

    // Swap MM between two resources, unassign and try repair tasks
    fn swap_mm(&mut self) -> f64 {
        unimplemented!()
    }


}
