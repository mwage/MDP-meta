use super::*;
use rand::{Rng, prelude::*};

pub struct MoveMMToAny {
    repair: bool
}

impl MoveMMToAny {
    pub fn new(repair: bool) -> Self {
        MoveMMToAny { repair }
    }
}

impl NeighborhoodFunction for MoveMMToAny {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();
        let mm = state.get_rand_mm();
        if mm.is_none() { return (0.0, change_tokens) }  // No major maintenance assigned

        let (res, time) = mm.unwrap();
        let length = state.instance().duration_major();
        // println!("{}: {:?}", res, state);
        let windows = state.get_all_suitable_windows_on_res(res, length, state.instance().horizon(), length, true);
        // println!("{}: {:?}", res, windows);
        if windows.is_empty() { return (0.0, change_tokens) } // Cannot move selected MM

        // Get new random time and add MM
        let mut rng = thread_rng();
        let (left, right) = windows.choose(&mut rng).unwrap();
        let new_time = rng.gen_range(*left..*right+1);

        // Replace reg maintenance
        // println!("MM: res {}: {}->{}", res, time, new_time);
        state.remove_major_maintenance(res);
        state.add_major_maintenance(res, new_time);
        change_tokens.push(ChangeToken::MovedMM(res, time));
        
        // Repair a task that was uncovered due to move
        if self.repair {
            if let Some(new_rm) = state.repair_after_move_any(res, time) {
                change_tokens.push(ChangeToken::AddRM(res, new_rm));
            }
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for MoveMMToAny {
    fn to_string(&self) -> String {
        format!("Move Major To Any ({})", if self.repair { "rep"} else { "norep" })
    }
}