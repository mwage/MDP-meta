use crate::instance::Instance;

use super::State;

pub struct Neighborhood {
    state: State
}

impl Neighborhood {
    pub fn new(instance: Instance) -> Self {
        let mut state = State::new(instance);
        state.initialize();

        Neighborhood { state }
    }

    pub fn state(&self) -> &State {
        &self.state
    }
    
    pub fn get_next(&mut self) -> f64 {
        unimplemented!()
    }

    pub fn undo_move(&mut self) {
        unimplemented!()
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PenaltyToken {
    Task(usize),
    MajMaint,
    RegMaintNotCovered(usize)
}

impl PenaltyToken {
    pub fn to_penalty(&self, instance: &Instance, multi: usize) -> usize {
        let maint_multi = 2;
        multi * match self {
            PenaltyToken::MajMaint => maint_multi * instance.duration_major(),
            PenaltyToken::RegMaintNotCovered(x) => *x,
            PenaltyToken::Task(i) => instance.tasks()[*i].length()
        }
    }
}