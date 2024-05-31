use bit_vec::BitVec;
use rand::{rngs::ThreadRng, Rng};
use crate::instance::Instance;
use super::instance::Task;
use rand::prelude::*;

use std::{collections::{BTreeMap, BTreeSet, VecDeque}, result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobToken {
    Task(usize),
    RegMaint,
    MajMaint
}

#[derive(Debug)]
pub struct State {
    // Currently unassigned jobs:
    instance: Instance,
    assigned_tasks: BitVec,
    assigned_maj_maint: BitVec,
    // Current solution state:
    jobs: Vec<BTreeMap<usize, JobToken>>,
    maj_maint_ends: Vec<usize>,
    reg_maint_ends: Vec<BTreeSet<usize>>,
    num_reg_maint: Vec<usize>,
    maintenance_changes: BTreeMap<usize, ChangeTimestamp>, // Number of maintenences after the timestamp
    obj_value: usize
}

impl State {
    pub fn new(instance: Instance) -> Self {
        let res = instance.resources();
        let assigned_tasks = BitVec::from_elem(instance.tasks().len(), false);
        let assigned_maj_maint = BitVec::from_elem(res, false);

        State {
            instance,
            assigned_tasks,
            assigned_maj_maint,
            jobs: vec![BTreeMap::new(); res],
            maj_maint_ends: vec![0; res],
            reg_maint_ends: vec![BTreeSet::new(); res],
            num_reg_maint: vec![0; res],
            maintenance_changes: BTreeMap::new(),
            obj_value: 0
        }
    }

    pub fn initialize(&mut self) {
        let mut rng = rand::thread_rng();

        for res in 0..self.instance.resources() {
            loop {
                let end_time = rng.gen_range(self.instance.duration_major()..self.instance.horizon());
                if !self.overlaps_other_mm(res, end_time) {
                    self.add_major_maintenence(res, end_time);
                    break;
                }                
            }
        }

        for task_idx in 0..self.instance.tasks().len() {
            let mut order: Vec<usize> = (0..self.instance.resources()).collect();
            order.shuffle(&mut rng);
            for &res in order.iter() {
                println!("try add task {} to res {}", task_idx, res);
                if self.can_add_task(res, task_idx) {
                    self.add_task(res, task_idx);
                    break;
                }
            }
        }

        for res in 0..self.instance.resources() {
            let mut to_add = Vec::new();
            let mut last_added = 2 * self.instance.horizon();
            for (&time, token) in self.jobs[res].iter().rev() {
                if time > last_added || time < self.instance.time_regular() {
                    continue;
                }
                println!("Try fix res {} time {}", res, time);
                match token {
                    JobToken::Task(_) => {
                        let limit = time - self.instance.time_regular();
                        if self.jobs[res].range(limit..time).any(|x| x.1 == &JobToken::MajMaint || x.1 == &JobToken::RegMaint) {
                            // Maintanence ended within time frame
                            continue;
                        }

                        // Add reg maintenance greedily at first suitable position
                        let mut possible_start = limit - self.instance.duration_regular();
                        let mut success = false;
                        for (&job_finished, token) in self.jobs[res].range(possible_start..time) {
                            let start = match token {
                                JobToken::MajMaint => job_finished - self.instance.duration_major(),
                                JobToken::RegMaint => job_finished - self.instance.duration_regular(),
                                JobToken::Task(i) => self.instance.tasks()[*i].start(),
                            };
                            if start < possible_start || start - possible_start < self.instance.duration_regular() {
                                possible_start = job_finished;
                                continue;
                            }
                            // Found a suitable slot
                            success = true;
                        }
                        if success {
                            last_added = possible_start + self.instance.duration_regular();
                            println!("Added at {}", last_added);
                            to_add.push(last_added);
                        } else {
                            // TODO: Penalties for missing maint
                            eprintln!("No maint slot");
                        }

                    },
                    _ => {}
                };
            }
            for end_time in to_add {
                self.add_regular_maintenance(res, end_time);
            }
        }
    }

    pub fn overlaps_other_mm(&self, resource: usize, end_time: usize) -> bool {
        self.maj_maint_ends.iter().enumerate().any(|(res, end)| res != resource && (*end as isize - end_time as isize).abs() < self.instance.duration_major() as isize)
    }

    pub fn can_add_task(&self, resource: usize, task_idx: usize) -> bool {
        let start = self.instance.tasks()[task_idx].start();
        let end = self.instance.tasks()[task_idx].end();
        let overlap_before = match self.jobs[resource].range(..end).next_back() {
            Some((time, _)) => *time > start,
            None => false
        };
        let overlap_after = match self.jobs[resource].range(start..).next() {
            Some((time, JobToken::MajMaint)) => end > *time - self.instance.duration_major(),
            Some((time, JobToken::RegMaint)) => end > *time - self.instance.duration_regular(),
            Some((_, JobToken::Task(i))) => end > self.instance.tasks()[*i].start(),
            None => false
        };

        !overlap_before && !overlap_after
    }

    pub fn add_task(&mut self, resource: usize, task_idx: usize) {
        self.assigned_tasks.set(task_idx, true);
        self.jobs[resource].insert(self.instance.tasks()[task_idx].end(), JobToken::Task(task_idx));
    }

    pub fn remove_task(&mut self, resource: usize, task_idx: usize) {
        self.assigned_tasks.set(task_idx, false);
        self.jobs[resource].remove(&self.instance.tasks()[task_idx].end());
    }

    pub fn add_major_maintenence(&mut self, res: usize, end_time: usize) {
        let start_time = end_time - self.instance.duration_major();
        self.assigned_maj_maint.set(res, true);
        self.maj_maint_ends[res] = end_time;
        self.jobs[res].insert(end_time, JobToken::MajMaint);
        
        self.update_changes_maint_added(start_time, end_time);
    }

    pub fn remove_major_maintenence(&mut self, res: usize) {
        let end_time = self.maj_maint_ends[res];
        let start_time = end_time - self.instance.duration_major();
        self.assigned_maj_maint.set(res, false);
        self.maj_maint_ends[res] = 0;
        self.jobs[res].remove(&end_time);

        self.update_changes_maint_removed(start_time, end_time);
    }

    pub fn add_regular_maintenance(&mut self, res: usize, end_time: usize) {
        let start_time = end_time - self.instance.duration_regular();
        self.reg_maint_ends[res].insert(end_time);
        self.num_reg_maint[res] += 1;
        self.jobs[res].insert(end_time, JobToken::RegMaint);
        
        self.update_changes_maint_added(start_time, end_time);
    }

    pub fn remove_regular_maintenence(&mut self, res: usize, idx: usize) {
        let end_time = *self.reg_maint_ends[res].iter().skip(idx).next().unwrap();
        let start_time = end_time - self.instance.duration_regular();
        self.reg_maint_ends[res].remove(&idx);
        self.num_reg_maint[res] -= 1;
        self.jobs[res].remove(&end_time);

        self.update_changes_maint_removed(start_time, end_time);
    }

    fn update_changes_maint_added(&mut self, start_time: usize, end_time: usize) {
        // Update maintenance changes
        let num_before_start = match self.maintenance_changes.range(..start_time).next_back() {
            Some(x) => x.1.count,
            None => 0
        };        
        let num_before_end = match self.maintenance_changes.range(..end_time).next_back() {
            Some(x) => x.1.count,
            None => 0
        };
        self.maintenance_changes.entry(start_time).or_insert(ChangeTimestamp::new(1, num_before_start));
        let end = self.maintenance_changes.entry(end_time).or_insert(ChangeTimestamp::new(0, num_before_end));
        end.num_changed += 1;
        for (_, stamp) in self.maintenance_changes.range_mut(start_time..end_time) {
            stamp.count += 1;
        }
        // Update obj valuee
        // println!("{:?}", self.maintenance_changes);
        let mut prev = (start_time, *self.maintenance_changes.get(&start_time).unwrap());
        let mut change = 0;
        // println!("{:?}", prev);
        for (&curr, stamp) in self.maintenance_changes.range(start_time+1..end_time+1) {
            let dist = curr - prev.0;
            change += (prev.1.count * prev.1.count - (prev.1.count - 1) * (prev.1.count - 1)) * dist;
            prev = (curr, *stamp)
        }
        self.obj_value += change;
    }

    fn update_changes_maint_removed(&mut self, start_time: usize, end_time: usize) {
        // Update maintenance changes
        for (_, stamp) in self.maintenance_changes.range_mut(start_time..end_time) {
            stamp.count -= 1;
        }
        self.maintenance_changes.get_mut(&start_time).unwrap().num_changed -= 1;
        self.maintenance_changes.get_mut(&end_time).unwrap().num_changed -= 1;

        // Update obj valuee
        // println!("{:?}", self.maintenance_changes);
        let mut prev = (start_time, *self.maintenance_changes.get(&start_time).unwrap());
        let mut change = 0;
        // println!("{:?}", prev);
        for (&curr, stamp) in self.maintenance_changes.range(start_time+1..end_time+1) {
            let dist = curr - prev.0;
            change += ((prev.1.count + 1) * (prev.1.count + 1) - prev.1.count * prev.1.count) * dist;
            prev = (curr, *stamp)
        }
        self.obj_value -= change;

        if self.maintenance_changes.get_mut(&start_time).unwrap().num_changed == 0 {
            self.maintenance_changes.remove(&start_time);
        };        
        if self.maintenance_changes.get_mut(&end_time).unwrap().num_changed == 0 {
            self.maintenance_changes.remove(&end_time);
        };
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChangeTimestamp {
    pub num_changed: usize,
    pub count: usize
}

impl ChangeTimestamp {
    pub fn new(num_changed: usize, count: usize) -> Self{
        ChangeTimestamp { num_changed, count }
    }
}