mod instance;
mod state;
mod simulated_annealing;
mod neighborhood;

use std::fs;

use instance::Instance;
use neighborhood::Neighborhood;
use simulated_annealing::{SAParameters, SimulatedAnnealing};
use state::State;


fn main() {
    // for path in fs::read_dir("./instances").unwrap() {
    //     let instance_name = path.unwrap().path().to_str().unwrap().to_string();
    //     let instance = Instance::new_from_file(&instance_name);
    //     let instance_name = instance_name.split("\\").last().unwrap().split(".").next().unwrap();
    // }

    let runs = 1;

    let mut counter = 0;
    let mut best_obj = usize::MAX;
    let mut total_working = 0;
    let mut total_feasible = 0;
    let mut total_iterations_since_accept = 0;
    let mut total_iterations_since_improvement = 0;
    for _ in 0..runs {
            
        let instance = Instance::new_from_file("./instances/mdp-3-7-5.json");
        let instance_name = "mdp-3-7-5".to_string();


        let mut sa = SimulatedAnnealing::new(Neighborhood::new(instance), SAParameters::default());
        sa.set_iterations(1000000);
        println!("Start: {} feasible: {} with {}({})", instance_name, sa.neighborhood().state().is_feasible(false), sa.neighborhood().state().obj_value(), sa.neighborhood().state().working_obj_val() - sa.neighborhood().state().obj_value());
        // println!("{:?}",sa.neighborhood().state());
        let (_, since_accept, since_impr) = sa.solve();
        total_iterations_since_accept += since_accept;
        total_iterations_since_improvement += since_impr;

        // Update stats
        total_working += sa.neighborhood().state().working_obj_val();
        let best = sa.get_best();
        if !best.is_none() { 
           // Feasible found
           let (obj, state) = best.as_ref().unwrap();        
           counter += 1;
           total_feasible += *obj;
           if *obj < best_obj {
               best_obj = *obj;
           }
        }

        let state = sa.neighborhood().state();
        println!("After: {} feasible: {} with {}({})", instance_name, state.is_feasible(false), state.obj_value(), state.working_obj_val() - state.obj_value());
        println!("{:?}", state);


        // eprintln!("{}", obj);


// //     // println!("Start instance {}", instance_name);
//     let mut neighborhood = Neighborhood::new(instance);
// //     // println!("Finished feasibility");
//     // println!("Start: {} feasible: {} with {}({})", instance_name, neighborhood.state().is_feasible(false), neighborhood.state().obj_value(), neighborhood.state().working_obj_val() - neighborhood.state().obj_value());
// //     // println!("{:?}", neighborhood.state().jobs());
    
//     for i in 0..100000 {
//         // println!("{:?}", neighborhood.state().jobs());
//         // for res in 0..neighborhood.state().instance().resources() {
//         //     let length = neighborhood.state().instance().duration_major();
//         //     println!("{:?}", neighborhood.state().get_all_suitable_windows_on_res(res, length, neighborhood.state().instance().horizon(), length));
//         // }

//         // println!("{:?}", neighborhood.state().jobs());
//         neighborhood.get_next();
//         // println!("{:?}", neighborhood.state().uncovered());
//         // neighborhood.reject();
//         let working_penalty = neighborhood.state().working_obj_val() - neighborhood.state().obj_value();
//         let actual_penalty = neighborhood.state().calc_penalty_from_scratch();
//         // println!("{}: {} feasible: {} with {}({})", i, instance_name, neighborhood.state().is_feasible(false), neighborhood.state().obj_value(), working_penalty);
        
//         if working_penalty != actual_penalty {
//             println!("Penalties not matching: working: {}, actual: {}, diff = {}", working_penalty, actual_penalty, working_penalty.abs_diff(actual_penalty));
//             println!("{:?}", neighborhood.state());
//             panic!("Penalties wrong");
//         }
        
//         if !neighborhood.state().is_feasible(false) {
//             println!("Infeasible assignments!");
//             println!("{:?}", neighborhood.state());
//             panic!("Infeasible");
//         }        
//         // println!("---------------------------------------------------------------------------------------------");
//         // println!("{:?}", neighborhood.state());
//     }

//     println!("After: {} feasible: {} with {}({})", instance_name, neighborhood.state().is_feasible(false), neighborhood.state().obj_value(), neighborhood.state().working_obj_val() - neighborhood.state().obj_value());
//     println!("{:?}", neighborhood.state().jobs());


    }

    println!("{}, best: {}, avg: {}, avg working: {}, since acc: {}, since impr: {}", counter, best_obj, total_feasible as f64 / counter as f64, total_working as f64 / runs as f64, total_iterations_since_accept as f64 / runs as f64, total_iterations_since_improvement as f64 / runs as f64);
}
