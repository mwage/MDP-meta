use super::*;
use rand::{Rng, prelude::*};

pub struct RemoveRM {
    repair: bool
}

impl RemoveRM {
    pub fn new(repair: bool) -> Self {
        RemoveRM { repair }
    }
}

impl NeighborhoodFunction for RemoveRM {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();
        let rm = state.get_rand_rm();
        if rm.is_none() { return (0.0, change_tokens) }  // No regular maintenance

        let (res, time) = rm.unwrap();
        state.remove_regular_maintenance(res, time);
        change_tokens.push(ChangeToken::RemoveRM(res, time));
        
        // Repair a task that was uncovered due to move
        if self.repair { 
            if let Some(new_rm) = state.repair_after_remove(res, time) {
                change_tokens.push(ChangeToken::AddRM(res, new_rm));
            }
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for RemoveRM {
    fn to_string(&self) -> String {
        format!("Remove Regular ({})", self.repair)
    }
}