use std::cmp;

use super::*;
use rand::{Rng, prelude::*};

pub struct MoveMM {
    repair: bool,
    max_move: usize
}

impl MoveMM {
    pub fn new(repair: bool, max_move: usize) -> Self {
        MoveMM { repair, max_move }
    }
}

impl NeighborhoodFunction for MoveMM {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();
        let mm = state.get_rand_mm();
        if mm.is_none() { return (0.0, change_tokens) }  // No major maintenance
        
        let (res, time) = mm.unwrap();
        let (left, right) = state.get_neighbors(res, time, true);
        if left == right { return (0.0, change_tokens) } // Cannot move selected RM

        // Get new random time and add MM
        let left = cmp::max(left, time-self.max_move);
        let right = cmp::min(right, time+self.max_move);
        let new_time = thread_rng().gen_range(left..right+1);

        // Replace maj maintenance
        // println!("MM: {}->{}", time, new_time);
        state.remove_major_maintenance(res);
        state.add_major_maintenance(res, new_time);
        change_tokens.push(ChangeToken::MovedMM(res, time));
        
        // Repair a task that was uncovered due to move
        if self.repair { 
            if let Some(new_rm) = state.repair_after_move(res, time, new_time) {
                change_tokens.push(ChangeToken::AddRM(res, new_rm));
            }
        }

        ((state.working_obj_val() - obj_prev) as f64, change_tokens)
    }
}

impl ToString for MoveMM {
    fn to_string(&self) -> String {
        format!("Move Major ({})", self.repair)
    }
}