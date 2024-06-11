mod neighborhood;
mod move_mm;
mod move_mm_to_any;
mod move_rm;
mod move_rm_to_any;
mod remove_rm;
mod cover_task;
mod add_task;
mod remove_task;

use super::{State, Instance};

use move_mm::MoveMM;
use move_mm_to_any::MoveMMToAny;
use move_rm::MoveRM;
use move_rm_to_any::MoveRMToAny;
use remove_rm::RemoveRM;
use cover_task::CoverTask;
use add_task::AddTask;
use remove_task::RemoveTask;

pub use neighborhood::Neighborhood;

pub trait NeighborhoodFunction: ToString {
    fn get_neighbor(&self, state: &mut State) -> (f64, Vec<ChangeToken>);
}

pub enum ChangeToken {
    MovedRM(usize, usize, usize),   // (res, prev, new)
    AddRM(usize, usize),   // (res, time)
    RemoveRM(usize, usize), // (res, time)
    AddMM(usize),   // (res)
    MovedMM(usize, usize),   // (res, prev)
    RemoveMM(usize, usize), // (res, time)
    AddTask(usize), // (task_id)
    RemoveTask(usize, usize), // (res, task_id)
    // AddMM(usize, usize),   // (res, time)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PenaltyToken {
    Task(usize),    // Task unassigned (task id)
    MajMaint,       // Maj maint unassigned
    RegMaintNotCovered(usize)   // Task (partially) uncovered (length that is uncovered)
}

impl PenaltyToken {
    pub fn to_penalty(&self, instance: &Instance, multi: usize) -> usize {
        let maint_multi = 2;
        multi * match self {
            PenaltyToken::MajMaint => maint_multi * instance.duration_major(),
            // PenaltyToken::MajMaint => 0,
            PenaltyToken::RegMaintNotCovered(x) => *x,  // Does not scale as bad as non-assigned tasks/maint
            // PenaltyToken::Task(i) => 0
            PenaltyToken::Task(i) => instance.tasks()[*i].length()
        }
    }
}
