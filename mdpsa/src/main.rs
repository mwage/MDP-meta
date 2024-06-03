mod instance;
mod state;
mod simulated_annealing;
mod neighborhood;

use std::fs;

use instance::Instance;
use neighborhood::Neighborhood;
use state::State;


fn main() {
    for path in fs::read_dir("./instances").unwrap() {
        let instance_name = path.unwrap().path().to_str().unwrap().to_string();
        let instance = Instance::new_from_file(&instance_name);
        let instance_name = instance_name.split("\\").last().unwrap().split(".").next().unwrap();


    }
    
    let instance = Instance::new_from_file("./instances/mdp-3-7-5.json");
    let instance_name = "mdp-3-7-5".to_string();

    // println!("Start instance {}", instance_name);
    let mut neighborhood = Neighborhood::new(instance);
    // println!("Finished feasibility");
    println!("{:?}", neighborhood.state());
    println!("{} feasible: {} with {}", instance_name, neighborhood.state().is_feasible(false), neighborhood.state().obj_value());
}
