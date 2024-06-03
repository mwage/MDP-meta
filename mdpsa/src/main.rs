mod instance;
mod state;
mod simulated_annealing;

use std::fs;

use instance::Instance;
use state::State;


fn main() {
    // for path in fs::read_dir("./instances").unwrap() {
    //     let instance = Instance::new_from_file(path.unwrap().path().to_str().unwrap());
    // }
    
    let test_instance = Instance::new_from_file("./instances/mdp-3-7-5.json");
    let mut state = State::new(test_instance);
    state.initialize();
    
    println!("{:?}", state);
    println!("Feasible: {}", state.is_feasible(false));
}
