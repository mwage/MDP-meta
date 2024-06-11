use crate::state::JobToken;

use super::*;
use rand::{Rng, prelude::*};

/// Adds an unassigned task (greedily without destruction or forcibly)
pub struct AddTask {
    greedy: bool,
    repair: bool
}


impl AddTask {
    pub fn new(greedy: bool, repair: bool) -> Self {
        AddTask { 
            greedy,
            repair
        }
    }
}

impl NeighborhoodFunction for AddTask {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();

        let unassigned_task = state.get_rand_unassigned_task();
        if unassigned_task.is_none() { return (0.0, change_tokens) }  // No unassigned task
        
        let task_id = unassigned_task.unwrap();
        let task = &state.instance().tasks()[task_id];
        
        if self.greedy {
            let mut order = (0..state.instance().resources()).collect::<Vec<usize>>();
            order.shuffle(&mut thread_rng());
            for &res in order.iter() {
                if state.can_add_task(res, task_id) {
                    state.add_task(res, task_id);
                    change_tokens.push(ChangeToken::AddTask(task_id));
                    break;
                }
            }
        } else {
            // Add to random resource:
            let res = thread_rng().gen_range(0..state.instance().resources());
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
            state.add_task(res, task_id);
            change_tokens.push(ChangeToken::AddTask(task_id));
        }
        
        if self.repair {
            change_tokens.append(&mut state.repair());
        }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for AddTask {
    fn to_string(&self) -> String {
        format!("Add task ({}, {})", if self.greedy { "greedy"} else { "random" }, if self.repair { "rep"} else { "norep" })
    }
}