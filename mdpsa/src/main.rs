mod instance;
mod state;
mod simulated_annealing;
mod neighborhood;

use std::{fs, time::Instant};

use crossbeam_utils::thread;


use instance::Instance;
use neighborhood::Neighborhood;
use simulated_annealing::{SAParameters, SimulatedAnnealing};
use state::State;


fn main() {
    let runs = 10;
    let timeout = 10*60*1000;   // 10 minutes
    println!("instance, min, avg, num_feasible, iterations, iterations_since_accept, iterations_since_improvement, runtime");
    for path in fs::read_dir("./instances").unwrap() {
        // let instance = Instance::new_from_file("./instances/mdp-3-7-5.json");

        let instance_name = path.unwrap().path().to_str().unwrap().to_string();
        let instance = Instance::new_from_file(&instance_name);
        let instance_name = instance_name.split("\\").last().unwrap().split(".").next().unwrap();
        println!("{}", results_to_string(run_multithreaded(instance, runs, timeout), &instance_name));
    }
}

fn run_multithreaded(instance: Instance, runs: usize, timeout: usize) -> Vec<Result> {
    thread::scope(|s| {
        let mut handles = Vec::new();
        for _ in 0..runs {
            let instance_clone = instance.clone();
            handles.push(s.spawn(move |_| {
                run_instance(instance_clone, timeout)
            }));
        }
        let mut results = Vec::new();
        for handle in handles.into_iter() {
            results.push(handle.join().unwrap());
        }
        results
    }).unwrap()
}

fn run_instance(instance: Instance, timeout: usize) -> Result {
    let test_iterations = 100000;
    let mut sa = SimulatedAnnealing::new(Neighborhood::new(instance), SAParameters::default());
    // Estimate iterations for timeout
    sa.set_iterations(test_iterations);
    let timer = Instant::now();
    let (iterations, iterations_since_accept, iterations_since_improvement) = sa.solve();
    let prep_time = Instant::now().duration_since(timer).as_millis() as usize;
    if prep_time > timeout {
        // timelimit already used up
        return Result::new(sa.get_best().clone(), iterations, iterations_since_accept, iterations_since_improvement, Instant::now().duration_since(timer).as_secs() as usize);
    }
    let iterations = timeout / prep_time * test_iterations;
    sa.set_iterations(iterations);
    sa.reset();

    // Solve instance
    let timer = Instant::now();
    let (iterations, iterations_since_accept, iterations_since_improvement) = sa.solve();

    Result::new(sa.get_best().clone(), iterations, iterations_since_accept, iterations_since_improvement, Instant::now().duration_since(timer).as_secs() as usize)
}

fn results_to_string(results: Vec<Result>, instance: &str) -> String {
    let num_feasible = results.iter().filter(|res| res.is_feasible()).count();
    let iterations = results.iter().fold(0, |acc, res| acc + res.iterations) / results.len();
    let iterations_since_accept = results.iter().fold(0, |acc, res| acc + res.iterations_since_accept()) / results.len();
    let iterations_since_improvement = results.iter().fold(0, |acc, res| acc + res.iterations_since_improvement()) / results.len();
    let runtime = results.iter().fold(0, |acc, res| acc + res.runtime()) / results.len();

    if num_feasible == 0 {
        return format!("{}, {}, {}, {}, {}, {}, {}, {}", instance, "-", "-", num_feasible, iterations, iterations_since_accept, iterations_since_improvement, runtime);
    }
    let obj_vals = results.iter().filter(|res| res.is_feasible()).map(|res| res.obj_val().unwrap()).collect::<Vec<usize>>();
    let min = obj_vals.iter().min().unwrap();
    let avg = obj_vals.iter().fold(0, |acc, time| acc + *time) / num_feasible;
    
    format!("{}, {}, {}, {}, {}, {}, {}, {}", instance, min, avg, num_feasible, iterations, iterations_since_accept, iterations_since_improvement, runtime)
}

pub struct Result {
    best: Option<(usize, State)>,
    iterations: usize,
    iterations_since_accept: usize,
    iterations_since_improvement: usize,
    runtime: usize
}

impl Result {
    pub fn new(best: Option<(usize, State)>, iterations: usize, iterations_since_accept: usize, iterations_since_improvement: usize, runtime: usize) -> Self {
        Result { best, iterations, iterations_since_accept, iterations_since_improvement, runtime }
    }

    pub fn obj_val(&self) -> Option<usize> {
        match self.best {
            Some((obj_val, _)) => Some(obj_val),
            None => None
        }
    }

    pub fn is_feasible(&self) -> bool {
        self.best.is_some()
    }

    pub fn iterations(&self) -> usize {
        self.iterations
    }

    pub fn iterations_since_improvement(&self) -> usize {
        self.iterations_since_improvement
    }

    pub fn iterations_since_accept(&self) -> usize {
        self.iterations_since_accept
    }

    pub fn runtime(&self) -> usize {
        self.runtime
    }
}
