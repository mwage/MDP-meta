use crate::state::JobToken;

use super::*;
use rand::{Rng, prelude::*};

pub struct AddMM {
    repair: bool
}

impl AddMM {
    pub fn new(repair: bool) -> Self {
        AddMM {
            repair
        }
    }
}

impl NeighborhoodFunction for AddMM {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();

        let unassigned_mm = state.get_rand_unassigned_mm();
        if unassigned_mm.is_none() { return (0.0, change_tokens) }  // No unassigned mm
        
        let res = unassigned_mm.unwrap();
        
        // Add at random time:
        let new_endtime = thread_rng().gen_range(state.instance().duration_major()..state.instance().horizon() + 1);

        // Remove all overlaps
        let overlaps = state.get_overlaps(res,new_endtime - state.instance().duration_major(), new_endtime);
        for (time, job) in overlaps.iter() {
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
        // Add new mm
        state.add_major_maintenance(res, new_endtime);
        change_tokens.push(ChangeToken::AddMM(res));
        
        if self.repair {
            // repair
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for AddMM {
    fn to_string(&self) -> String {
        format!("Add task ({})", if self.repair { "rep"} else { "norep" })
    }
}