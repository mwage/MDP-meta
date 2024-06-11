use super::*;

pub struct RemoveTask {}

impl RemoveTask {
    pub fn new() -> Self {
        RemoveTask {}
    }
}

impl NeighborhoodFunction for RemoveTask {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>) {
        let obj_prev = state.working_obj_val();
        let mut change_tokens = Vec::new();

        let assigned_task = state.get_rand_assigned_task();
        if assigned_task.is_none() { return (0.0, change_tokens) }  // No assigned task

        let (res, task_id) = assigned_task.unwrap();
        state.remove_task(task_id);
        change_tokens.push(ChangeToken::RemoveTask(res, task_id));

        ((state.working_obj_val() as isize - obj_prev as isize) as f64, change_tokens)
    }
}

impl ToString for RemoveTask {
    fn to_string(&self) -> String {
        format!("Remove task")
    }
}