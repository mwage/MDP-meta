use crate::state::JobToken;

use super::*;
use rand::{Rng, prelude::*};

pub struct MoveMMDestructive {
    repair: bool
}

impl MoveMMDestructive {
    pub fn new(repair: bool) -> Self {
        MoveMMDestructive { repair }
    }
}

impl NeighborhoodFunction for MoveMMDestructive {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();
        let mm = state.get_rand_mm();
        if mm.is_none() { return (0.0, change_tokens) }  // No major maintenance assigned

        let (res, mm_time) = mm.unwrap();
        // Add at random time:
        let new_endtime = thread_rng().gen_range(state.instance().duration_major()..state.instance().horizon() + 1);

        // Remove old mm
        state.remove_major_maintenance(res);
        change_tokens.push(ChangeToken::RemoveMM(res, mm_time));

        // Remove all overlaps
        for (time, job) in state.get_overlaps(res, new_endtime - state.instance().duration_major(), new_endtime).iter() {
            match job {
                JobToken::MajMaint => { panic!("Two mm on res?") },
                JobToken::RegMaint => {
                    state.remove_regular_maintenance(res, *time);
                    change_tokens.push(ChangeToken::RemoveRM(res, *time))
                },
                JobToken::Task(id) => {
                    state.remove_task(*id);
                    change_tokens.push(ChangeToken::RemoveTask(res, *id))
                }
            }
        }
        let mm_overlaps = state.get_other_mm_overlaps(res, new_endtime);
        for (r, time) in mm_overlaps {
            state.remove_major_maintenance(r);
            change_tokens.push(ChangeToken::RemoveMM(r, time));
        }

        // Add new major maintenance
        state.add_major_maintenance(res, new_endtime);
        change_tokens.push(ChangeToken::AddMM(res));
        
        // Repair a task that was uncovered due to move
        if self.repair {
            change_tokens.append(&mut state.repair());
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for MoveMMDestructive {
    fn to_string(&self) -> String {
        format!("Move Major destructively ({})", if self.repair { "rep"} else { "norep" })
    }
}