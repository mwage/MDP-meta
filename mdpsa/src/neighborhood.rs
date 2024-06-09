use crate::instance::Instance;

use super::State;

use rand::{rngs::ThreadRng, Rng};
use rand::prelude::*;

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
        // TODO: Select (randomly?) from neighborhoods
        self.move_rm(false)
    }

    pub fn accept(&mut self) {
        self.last_changes = Vec::new();
    }

    pub fn reject(&mut self) {
        for token in self.last_changes.iter() {
            match token {
                ChangeToken::MovedRM(res, prev, new) => {
                    self.state.remove_regular_maintenance(*res, *new);
                    self.state.add_regular_maintenance(*res, *prev);
                }
            }
        }
    }
    
    fn instance(&self) -> &Instance {
        &self.state.instance()
    }

    // Move a regular maintenance within boundries between neighbors
    fn move_rm(&mut self, repair: bool) -> f64 {
        let rm = self.state.get_rand_rm();
        if rm.is_none() { return 0.0 }  // No regular maintenance

        let (res, time) = rm.unwrap();
        let (left, right) = self.state.get_neighbors(res, time, false);
        if left == right { return 0.0 } // Cannot move selected RM

        // Get new random time and add RM
        let obj_prev = self.state.working_obj_val();
        let mut rng = thread_rng();
        let new_time = rng.gen_range(left..right+1);

        // Replace reg maintenance
        // println!("{}->{}", time, new_time);
        self.state.remove_regular_maintenance(res, time);
        self.state.add_regular_maintenance(res, new_time);
        self.last_changes.push(ChangeToken::MovedRM(res, time, new_time));
        
        // Repair a task that was uncovered due to move
        if repair { 
            self.state.repair_after_move(res, time, new_time) 
        }

        (self.state.working_obj_val() - obj_prev) as f64
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

pub enum ChangeToken {
    MovedRM(usize, usize, usize)   // (res, prev, new)
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PenaltyToken {
    Task(usize),
    MajMaint,
    RegMaintNotCovered(usize)
}

impl PenaltyToken {
    pub fn to_penalty(&self, instance: &Instance, multi: usize) -> usize {
        let maint_multi = 2;
        multi * match self {
            // PenaltyToken::MajMaint => maint_multi * instance.duration_major(),
            PenaltyToken::MajMaint => 0,
            PenaltyToken::RegMaintNotCovered(x) => *x,  // Does not scale as bad as non-assigned tasks/maint
            PenaltyToken::Task(i) => 0
            // PenaltyToken::Task(i) => instance.tasks()[*i].length()
        }
    }
}