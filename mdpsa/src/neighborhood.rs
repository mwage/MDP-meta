use crate::instance::Instance;

use super::State;

use rand::{rngs::ThreadRng, Rng};
use rand::prelude::*;

pub struct Neighborhood {
    state: State,
    last_changes: Vec<ChangeToken>,
    curr_penalty: usize
}

impl Neighborhood {
    pub fn new(instance: Instance) -> Self {
        let curr_penalty = 1;
        let mut state = State::new(instance);
        state.initialize(curr_penalty);

        Neighborhood { 
            state,
            last_changes: Vec::new(),
            curr_penalty
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    fn instance(&self) -> &Instance {
        &self.state.instance()
    }
    
    pub fn get_next(&mut self) -> f64 {
        self.move_rm(false);
        unimplemented!()
    }

    pub fn undo_move(&mut self) {
        unimplemented!()
    }

    pub fn move_rm(&mut self, repair: bool) -> f64 {
        let rm = self.state.get_rand_rm();
        if rm.is_none() { return 0.0 }

        let (res, time) = rm.unwrap();
        let (left, right) = self.state.get_neighbors(res, time, false);
        if left == right { return 0.0 } // Cannot move

        // TODO: Make nicer move with check for 
        let obj_prev = self.state.working_obj_val();
        self.state.remove_regular_maintenence(res, time);
        let mut rng = thread_rng();

        let new_time = rng.gen_range(left..right+1);
        self.state.add_regular_maintenance(res, new_time);
        self.last_changes.push(ChangeToken::MovedRM(time, new_time));
        

        (self.state.working_obj_val() - obj_prev) as f64
    }
}

pub enum ChangeToken {
    MovedRM(usize, usize),
    NewUncovered(usize)
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
            PenaltyToken::MajMaint => maint_multi * instance.duration_major(),
            PenaltyToken::RegMaintNotCovered(x) => *x,
            PenaltyToken::Task(i) => instance.tasks()[*i].length()
        }
    }
}