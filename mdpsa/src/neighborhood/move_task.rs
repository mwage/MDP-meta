use crate::state::JobToken;

use super::*;
use rand::{Rng, prelude::*};

/// Adds an unassigned task (greedily without destruction or forcibly)
pub struct MoveTask {
    repair: bool
}


impl MoveTask {
    pub fn new(repair: bool) -> Self {
        MoveTask {
            repair
        }
    }
}

impl NeighborhoodFunction for MoveTask {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();

        let assigned_task = state.get_rand_assigned_task();
        if assigned_task.is_none() { return (0.0, change_tokens) }  // No assigned task
        
        let (prev_res, task_id) = assigned_task.unwrap();
        let task = &state.instance().tasks()[task_id];
        
        let mut res = thread_rng().gen_range(0..state.instance().resources() - 1);
        if res >= prev_res {
            res += 1;   // Shift to viable res idx
        }
            
        // Remove all overlaps
        let overlaps = state.get_overlaps(res, task.start(), task.end());
        for (time, job) in overlaps.iter() {
            match job {
                JobToken::MajMaint => {
                    state.remove_major_maintenance(res);
                    change_tokens.push(ChangeToken::RemoveMM(res, *time))
                },
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

        // Add new task
        state.remove_task(task_id);
        change_tokens.push(ChangeToken::RemoveTask(prev_res, task_id));
        state.add_task(res, task_id);
        change_tokens.push(ChangeToken::AddTask(task_id));
        
        if self.repair {
            change_tokens.append(&mut state.repair());
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for MoveTask {
    fn to_string(&self) -> String {
        format!("Move task ({})", if self.repair { "rep"} else { "norep" })
    }
}