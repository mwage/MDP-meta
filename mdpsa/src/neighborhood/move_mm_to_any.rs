use super::*;

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
        let new_time = state.can_add_mm_without_destruction(res);
        if new_time.is_none() { return (0.0, change_tokens) }

        // Replace reg maintenance
        state.remove_major_maintenance(res);
        state.add_major_maintenance(res, new_time.unwrap());
        change_tokens.push(ChangeToken::MovedMM(res, time));
        
        // Repair a task that was uncovered due to move
        if self.repair {
            change_tokens.append(&mut state.repair());
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for MoveMMToAny {
    fn to_string(&self) -> String {
        format!("Move Major To Any ({})", if self.repair { "rep"} else { "norep" })
    }
}