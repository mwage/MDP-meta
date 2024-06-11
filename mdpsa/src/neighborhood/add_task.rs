use super::*;
use rand::{Rng, prelude::*};

pub struct AddTask {
    greedy: bool
}

impl AddTask {
    pub fn new(greedy: bool) -> Self {
        AddTask { 
            greedy
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
        for res in 0..state.instance().resources() {
            if state.can_add_task(res, task_id) {
                // println!("{:?}", state.jobs());
                // println!("Add {} to res {}", task_id, res);
                state.add_task(res, task_id);
                change_tokens.push(ChangeToken::AddTask(task_id));
                break;
            }
        }
        
        // let task = state.instance().tasks()[task_id];
        

        // if self.greedy {
        //     // Cover greedily
        //     if let Some(new_rm) = state.find_reg_maint_cover_greedy(res, time) {
        //         state.add_regular_maintenance(res, new_rm);
        //         change_tokens.push(ChangeToken::AddRM(res, new_rm));
        //     }
        // } else {
        //     // Cover randomly
        //     // println!("{:?}", state);
        //     if let Some(new_rm) = state.find_reg_maint_cover_greedy(res, time) {
        //         // println!("Try add at res {}: {}", res, new_rm);
        //         state.add_regular_maintenance(res, new_rm);
        //         change_tokens.push(ChangeToken::AddRM(res, new_rm));
        //     }
        // }

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for AddTask {
    fn to_string(&self) -> String {
        format!("Add task ({})", if self.greedy { "greedy"} else { "random" })
    }
}