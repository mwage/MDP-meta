use super::*;
use rand::{Rng, prelude::*};

pub struct MoveRMToAny {
    repair: bool
}

impl MoveRMToAny {
    pub fn new(repair: bool) -> Self {
        MoveRMToAny { repair }
    }
}

impl NeighborhoodFunction for MoveRMToAny {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();
        let rm = state.get_rand_rm();
        if rm.is_none() { return (0.0, change_tokens) }  // No regular maintenance

        let (res, time) = rm.unwrap();
        let length = state.instance().duration_regular();
        let windows = state.get_all_suitable_windows_on_res(res, length, state.instance().horizon(), length);
        // println!("{:?}", windows);
        if windows.is_empty() { return (0.0, change_tokens) } // Cannot move selected RM

        // Get new random time and add RM
        let mut rng = thread_rng();
        let (left, right) = windows.choose(&mut rng).unwrap();
        let new_time = rng.gen_range(*left..*right+1);

        // Replace reg maintenance
        // println!("RM: res {}: {}->{}", res, time, new_time);
        state.remove_regular_maintenance(res, time);
        state.add_regular_maintenance(res, new_time);
        change_tokens.push(ChangeToken::MovedRM(res, time, new_time));
        
        // Repair a task that was uncovered due to move
        if self.repair {
            if let Some(new_rm) = state.repair_after_move_any(res, time) {
                change_tokens.push(ChangeToken::AddRM(res, new_rm));
            }
        }

        ((state.working_obj_val() - obj_prev) as f64, change_tokens)
    }
}

impl ToString for MoveRMToAny {
    fn to_string(&self) -> String {
        format!("Move Regular To Any ({})", self.repair)
    }
}