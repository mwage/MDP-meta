use rand::Rng;
use super::neighborhood::Neighborhood;

use super::state::State;
use std::f64::consts::E;

pub struct SimulatedAnnealing {
    parameters: SAParameters,
    temperature: f64,
    neighborhood: Neighborhood,
    best_feasible: Option<(usize, State)>
}

impl SimulatedAnnealing {
    pub fn new(neighborhood: Neighborhood, parameters: SAParameters) -> Self {
        let temperature = parameters.initial_temperature();

        SimulatedAnnealing {
            parameters,
            temperature,
            neighborhood,
            best_feasible: None
        }
    }
    
    pub fn set_iterations(&mut self, iterations: usize) {
        self.parameters.set_alpha_to_iterations(iterations);
    }
    
    pub fn neighborhood(&self) -> &Neighborhood {
        &self.neighborhood
    }

    pub fn get_best(&self) -> &Option<(usize, State)> {
        &self.best_feasible
    }

    pub fn reset(&mut self) {
        self.temperature = self.parameters.initial_temperature();
    }
    
    fn decrease_temperature(&mut self) {
        self.temperature *= self.parameters.alpha()
    }

    fn accept(&self, delta: f64) -> bool {
        if delta <= 0f64 {
            return true;
        }
        // high delta = bad move = x small
        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() < E.powf(- delta / self.temperature)  {
            return true;
        }
        false
    }

    pub fn solve(&mut self) -> (usize, usize, usize) {
        let mut iterations = 0;
        let mut iterations_since_accept = 0;
        let mut iterations_since_improvement = 0;
        let mut best_obj = usize::MAX;
        while self.temperature > self.parameters.final_temperature() {
            let delta = self.neighborhood.get_next();
            let working_penalty = self.neighborhood.state().working_obj_val() - self.neighborhood.state().obj_value();
            let actual_penalty = self.neighborhood.state().calc_penalty_from_scratch();
            assert_eq!(working_penalty, actual_penalty);
            assert!(self.neighborhood.state().is_feasible(false));
            iterations_since_improvement += 1;
            iterations += 1;
            if self.accept(delta) {
                self.neighborhood.accept();
                if delta != 0.0 {
                    iterations_since_accept = 0;
                }
                let obj_val = self.neighborhood.state().working_obj_val();
                if obj_val < best_obj {
                    best_obj = obj_val;
                    iterations_since_improvement = 0;
                }
                let state = self.neighborhood().state();
                if state.is_feasible_quick() {
                    let obj = state.obj_value();
                    // Try update, but ensure feasibility
                    let add = match self.best_feasible {
                        Some((prev_best, _)) => obj < prev_best,
                        None => true
                    };
                    if add && state.is_feasible(true) {
                        self.best_feasible = Some((obj, state.clone()));
                    }
                }
            } else {
                self.neighborhood.reject();
                iterations_since_accept += 1;
            }

            self.decrease_temperature();
            // if iterations % (self.parameters.iterations() / self.parameters.max_penalty) == 0 {
            //     self.neighborhood.increase_penalty_multi();
            // }
        }

        (iterations, iterations_since_accept, iterations_since_improvement)
    }
}

pub struct SAParameters {
    alpha: f64,
    initial_temperature: f64,
    final_temperature: f64,
    max_penalty: usize,
    iterations: usize
}

impl SAParameters {
    pub fn new(initial_temperature: f64, final_temperature: f64, max_penalty: usize, iterations: usize) -> Self {
        SAParameters { alpha: 0.99, initial_temperature, final_temperature, max_penalty, iterations }
    }

    pub fn alpha(&self)-> f64 {
        self.alpha
    }

    pub fn max_penalty(&self)-> usize {
        self.max_penalty
    }

    pub fn initial_temperature(&self) -> f64 {
        self.initial_temperature
    }
    
    pub fn final_temperature(&self) -> f64 {
        self.final_temperature
    }
    pub fn set_alpha_to_iterations(&mut self, iterations: usize) {
        self.iterations = iterations;
        self.alpha = (self.final_temperature / self.initial_temperature).powf(1.0 / iterations as f64)
    }
}

impl Default for SAParameters {
    fn default() -> Self {
        SAParameters {
            alpha: 0.99,
            initial_temperature: 10000.0,
            final_temperature: 10.0,
            max_penalty: 10,
            iterations: 100000
        }
    }
}