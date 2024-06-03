use rand::Rng;
use super::neighborhood::Neighborhood;

use super::state::State;
use std::f64::consts::E;

pub struct SimulatedAnnealing {
    parameters: SAParameters,
    temperature: f64,
    neighborhood: Neighborhood
}

impl SimulatedAnnealing {
    pub fn new(neighborhood: Neighborhood, parameters: SAParameters) -> Self {
        let temperature = parameters.initial_temperature();

        SimulatedAnnealing {
            parameters,
            temperature,
            neighborhood
        }
    }
    
    fn decrease_temperature(&mut self) {
        self.temperature *= self.parameters.alpha
    }

    fn accept(&self, delta: f64) -> bool {
        if delta <= 0f64 {
            return true;
        }
        // high delta = bad move = x small
        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() < E.powf(- delta / self.temperature)  {
            // println!("accepting: {}", delta);
            return true;
        }
        // println!("reject: {}", delta);
        false
    }

    pub fn extract(self) -> (Neighborhood, SAParameters) {
        (self.neighborhood, self.parameters)
    }
    
    pub fn solve(mut self) {
        let mut iterations = 0;
        let mut iterations_since_accept = 0;
        let mut iterations_since_improvement = 0;
        let mut total_delta = 0f64;
        let mut best_delta = 0f64;
        while self.temperature > self.parameters.final_temperature() {
            let delta = self.neighborhood.get_next();
            iterations_since_improvement += 1;
            iterations += 1;
            if self.accept(delta) {
                total_delta += delta;
                iterations_since_accept = 0;
                if total_delta < best_delta {
                    best_delta = total_delta;
                    iterations_since_improvement = 0;
                    // self.neighborhood.set_best();
                }
            } else {
                self.neighborhood.undo_move();
                iterations_since_accept += 1;
            }

            self.decrease_temperature();
        }
        
        if total_delta > best_delta {
            // self.neighborhood.revert_to_best();
        }
    }
}

pub struct SAParameters {
    alpha: f64,
    initial_temperature: f64,
    final_temperature: f64,
    timelimit: u128
}

impl SAParameters {
    pub fn new(alpha: f64, initial_temperature: f64, final_temperature: f64, timelimit: u128) -> Self {
        SAParameters { alpha, initial_temperature, final_temperature, timelimit }
    }

    pub fn alpha(&self)-> f64 {
        self.alpha
    }

    pub fn initial_temperature(&self) -> f64 {
        self.initial_temperature
    }
    
    pub fn final_temperature(&self) -> f64 {
        self.final_temperature
    }
    pub fn timelimit(&self) -> u128 {
        self.timelimit
    }
}

impl Default for SAParameters {
    fn default() -> Self {
        SAParameters {
            alpha: 0.99,
            initial_temperature: 1000.0,
            final_temperature: 0.0001,
            timelimit: 1000*60*5
        }
    }
}