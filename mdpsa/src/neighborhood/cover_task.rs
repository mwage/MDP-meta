use super::*;
use rand::{Rng, prelude::*};

pub struct CoverTask {
    greedy: bool
}

impl CoverTask {
    pub fn new(greedy: bool) -> Self {
        CoverTask { 
            greedy
        }
    }
}

impl NeighborhoodFunction for CoverTask {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();
        let uncovered_task = state.get_rand_uncovered_task();
        if uncovered_task.is_none() { return (0.0, change_tokens) }  // No uncovered task
        
        let (res, time) = uncovered_task.unwrap();

        if self.greedy {
            // Cover greedily
            if let Some(new_rm) = state.find_reg_maint_cover_greedy(res, time) {
                state.add_regular_maintenance(res, new_rm);
                change_tokens.push(ChangeToken::AddRM(res, new_rm));
            }
        } else {
            // Cover randomly
            // println!("{:?}", state);
            if let Some(new_rm) = state.find_reg_maint_cover_greedy(res, time) {
                // println!("Try add at res {}: {}", res, new_rm);
                state.add_regular_maintenance(res, new_rm);
                change_tokens.push(ChangeToken::AddRM(res, new_rm));
            }
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for CoverTask {
    fn to_string(&self) -> String {
        format!("Cover task ({})", if self.greedy { "greedy"} else { "random" })
    }
}