use crate::state::JobToken;

use super::*;
use rand::{Rng, prelude::*};

pub struct SwapMM {
    repair: bool
}

impl SwapMM {
    pub fn new(repair: bool) -> Self {
        SwapMM { repair }
    }
}

impl NeighborhoodFunction for SwapMM {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();
        let mm = state.get_rand_mm();
        if mm.is_none() { return (0.0, change_tokens); }  // No major maintenance assigned

        let (res, mm_time) = mm.unwrap();
        // Get a second resource
        let other_ass_count = state.assigned_maj_maint().iter().filter(|b| *b).count() - 1;
        if other_ass_count == 0 { return (0.0, change_tokens); }    // No second major maintenance assigned
        let skip_idx = thread_rng().gen_range(0..other_ass_count);
        let mut other_res = 0;
        let mut counter = 0;
        for (i, _) in state.assigned_maj_maint().iter().enumerate().filter(|(_, b)| *b) {
            if i == res { continue; }

            if counter == skip_idx {
                other_res = i;
                break;
            }
            counter += 1;
        }
        let other_time = state.maj_maint_ends()[other_res];

        // Remove all overlaps from current res
        for (time, job) in state.get_overlaps(res, other_time - state.instance().duration_major(), other_time).iter() {
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
        // Remove all overlaps from other res
        for (time, job) in state.get_overlaps(other_res, mm_time - state.instance().duration_major(), mm_time).iter() {
            match job {
                JobToken::MajMaint => { panic!("Two mm on res?") },
                JobToken::RegMaint => {
                    state.remove_regular_maintenance(other_res, *time);
                    change_tokens.push(ChangeToken::RemoveRM(other_res, *time))
                },
                JobToken::Task(id) => {
                    state.remove_task(*id);
                    change_tokens.push(ChangeToken::RemoveTask(other_res, *id))
                }
            }
        }

        // Swap major maintenances
        state.remove_major_maintenance(res);
        state.remove_major_maintenance(other_res);
        change_tokens.push(ChangeToken::RemoveMM(res, mm_time));
        change_tokens.push(ChangeToken::RemoveMM(other_res, other_time));

        state.add_major_maintenance(res, other_time);
        state.add_major_maintenance(other_res, mm_time);
        change_tokens.push(ChangeToken::AddMM(res));
        change_tokens.push(ChangeToken::AddMM(other_res));
        
        // Repair a task that was uncovered due to move
        if self.repair {
            change_tokens.append(&mut state.repair());
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for SwapMM {
    fn to_string(&self) -> String {
        format!("Swap Major ({})", if self.repair { "rep"} else { "norep" })
    }
}