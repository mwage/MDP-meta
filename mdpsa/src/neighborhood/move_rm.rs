use std::cmp;

use super::*;
use rand::{Rng, prelude::*};

pub struct MoveRM {
    repair: bool,
    max_move: usize
}

impl MoveRM {
    pub fn new(repair: bool, max_move: usize) -> Self {
        MoveRM { repair, max_move }
    }
}

impl NeighborhoodFunction for MoveRM {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();
        let rm = state.get_rand_rm();
        if rm.is_none() { return (0.0, change_tokens) }  // No regular maintenance

        let (res, time) = rm.unwrap();
        let (left, right) = state.get_neighbors(res, time, false);
        if left == right { return (0.0, change_tokens) } // Cannot move selected RM

        // Get new random time and add RM
        let left = cmp::max(left, time-self.max_move);
        let right = cmp::min(right, time+self.max_move);
        let new_time = thread_rng().gen_range(left..right+1);
        
        // Replace reg maintenance
        state.remove_regular_maintenance(res, time);
        state.add_regular_maintenance(res, new_time);
        change_tokens.push(ChangeToken::MovedRM(res, time, new_time));
        
        // Repair a task that was uncovered due to move
        if self.repair { 
            change_tokens.append(&mut state.repair());
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for MoveRM {
    fn to_string(&self) -> String {
        format!("Move Regular ({})", if self.repair { "rep"} else { "norep" })
    }
}