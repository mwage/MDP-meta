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
    for j in 0..100{
    let instance = Instance::new_from_file("./instances/mdp-3-7-5.json");
    let instance_name = "mdp-3-7-5".to_string();

    // println!("Start instance {}", instance_name);
    let mut neighborhood = Neighborhood::new(instance);
    // println!("Finished feasibility");
    println!("Start: {} feasible: {} with {}({})", instance_name, neighborhood.state().is_feasible(false), neighborhood.state().obj_value(), neighborhood.state().working_obj_val() - neighborhood.state().obj_value());
    // println!("{:?}", neighborhood.state());
    
    for i in 0..100000 {
        // println!("{:?}", neighborhood.state().jobs());
        // for res in 0..neighborhood.state().instance().resources() {
        //     let length = neighborhood.state().instance().duration_major();
        //     println!("{:?}", neighborhood.state().get_all_suitable_windows_on_res(res, length, neighborhood.state().instance().horizon(), length));
        // }

        neighborhood.get_next();
        // println!("{:?}", neighborhood.state().jobs());
        // println!("{:?}", neighborhood.state().uncovered());
        // neighborhood.reject();
        // println!("{}: {} feasible: {} with {}({})", i, instance_name, neighborhood.state().is_feasible(false), neighborhood.state().obj_value(), neighborhood.state().working_obj_val() - neighborhood.state().obj_value());
        if !neighborhood.state().is_feasible(false) || neighborhood.state().working_obj_val() - neighborhood.state().obj_value() != neighborhood.state().calc_penalty_from_scratch() {
            println!("{:?}", neighborhood.state());
            panic!();
        }
        
        // println!("---------------------------------------------------------------------------------------------");
        // println!("{:?}", neighborhood.state());
    }
    println!("After: {} feasible: {} with {}({})", instance_name, neighborhood.state().is_feasible(false), neighborhood.state().obj_value(), neighborhood.state().working_obj_val() - neighborhood.state().obj_value());
    println!("{:?}", neighborhood.state().jobs());
}
}
