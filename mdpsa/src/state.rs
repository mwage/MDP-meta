use bit_vec::BitVec;
use crate::{instance::Instance, neighborhood::PenaltyToken};
use super::instance::Task;
use std::{collections::{BTreeMap, BTreeSet}, cmp};
use rand::{rngs::ThreadRng, Rng};
use rand::prelude::*;

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
    assigned_tasks: BitVec, // Which tasks are currently assigned
    assigned_maj_maint: BitVec, // Which maja maintenances are currently assigned
    // Current solution state:
    jobs: Vec<BTreeMap<usize, JobToken>>,   // All jobs currently assigned on each resource
    maj_maint_ends: Vec<usize>, // End times of all maj maintenances
    reg_maint_ends: Vec<BTreeSet<usize>>,   // End times of all regular maintenances on each resource
    task_ass: Vec<usize>,   // To which res a task is assigned to
    num_reg_maint: Vec<usize>,  // Number of reg maintenances per resource
    maintenance_changes: BTreeMap<usize, ChangeTimestamp>, // Number of maintenences after the timestamp
    obj_value: usize,   // Obj value of instance (without penalties)
    penalty_value: usize,   // Current penalty value (including modifier)
    uncovered: Vec<BTreeSet<usize>>,  // Uncovered tasks (end time of task), if you need ID -> get via jobs
    penalty_multi: usize // Current penalty modifier
}

impl State {
    pub fn new(instance: Instance, initial_penalty: usize) -> Self {
        let res = instance.resources();
        let assigned_tasks = BitVec::from_elem(instance.tasks().len(), false);
        let assigned_maj_maint = BitVec::from_elem(res, false);
        let task_ass = vec![usize::MAX; instance.tasks().len()];

        State {
            instance,
            assigned_tasks,
            assigned_maj_maint,
            jobs: vec![BTreeMap::new(); res],
            maj_maint_ends: vec![0; res],
            reg_maint_ends: vec![BTreeSet::new(); res],
            task_ass,
            num_reg_maint: vec![0; res],
            maintenance_changes: BTreeMap::new(),
            obj_value: 0, 
            penalty_value: 0,
            uncovered: vec![BTreeSet::new(); res],
            penalty_multi: initial_penalty
        }
    }

    pub fn jobs(&self) -> &Vec<BTreeMap<usize, JobToken>> {
        &self.jobs
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn obj_value(&self) -> usize {
        self.obj_value
    }

    pub fn working_obj_val(&self) -> usize {
        self.obj_value + self.penalty_value
    }

    pub fn assigned_tasks(&self) -> &BitVec {
        &self.assigned_tasks
    }

    pub fn assigned_maj_maint(&self) -> &BitVec {
        &self.assigned_maj_maint
    }

    pub fn uncovered(&self) -> &Vec<BTreeSet<usize>> {
        &self.uncovered
    }

    pub fn initialize(&mut self) {
        let mut penalties = Vec::new();
        // Add major maintenances at random (non-overlapping) times
        for res in 0..self.instance.resources() {
            self.add_major_maintenence(res, (res+1) * self.instance.duration_major());
        }

        // Assign tasks to a random (free) resource
        for task_idx in 0..self.instance.tasks().len() {
            match (0..self.instance.resources()).find(|res| self.can_add_task(*res, task_idx)) {
                Some(res) => { self.add_task(res, task_idx); },
                None => { penalties.push(PenaltyToken::Task(task_idx)) }
            };
        }

        // Add regular maintenances to cover any uncovered tasks (if possible)
        for res in 0..self.instance.resources() {
            let mut to_add = Vec::new();
            let mut last_added = 2 * self.instance.horizon();
            // Iterate jobs on resource in reverse order
            for (&time, token) in self.jobs[res].iter().rev() {
                if time > last_added || time < self.instance.time_regular() {
                    continue;
                }
                match token {
                    JobToken::Task(_) => {
                        match self.has_maint_covered(res, time) {
                            Some(x) => { last_added = x; continue; },
                            None => {}
                        };
                        
                        match self.find_reg_maint_cover_greedy(res, time) {
                            Some(x) => {
                                last_added = x;
                                to_add.push(x);
                            },
                            None => {
                                self.uncovered[res].insert(time);
                            }
                        }
                    },
                    _ => {}
                };
            }
            for end_time in to_add {
                self.add_regular_maintenance(res, end_time);
            }
            last_added = 2 * self.instance.horizon();
            // Add penalties for uncovered
            for (&time, token) in self.jobs[res].iter().rev() {
                if time > last_added || time < self.instance.time_regular() {
                    continue;
                }
                match token {
                    JobToken::Task(_) => {
                        last_added = match self.jobs[res].iter().rev().filter(|(x, _)| **x < time).find(|(_, token)| *token == &JobToken::MajMaint || *token == &JobToken::RegMaint) {
                            Some((x, _)) => *x,
                            None => 0
                        };
                        let diff = time - last_added;
                        if time - last_added > self.instance.time_regular() {
                            penalties.push(PenaltyToken::RegMaintNotCovered(diff - self.instance.time_regular()));
                        }
                    },
                    _ => last_added = time
                }
            }
        }
        // Apply penalties
        self.penalty_value += penalties.iter().map(|x| x.to_penalty(&self.instance, self.penalty_multi)).sum::<usize>();
    }

    pub fn is_feasible_quick(&self) -> bool {
        self.penalty_value == 0
    }

    pub fn is_feasible(&self, requires_completeness: bool) -> bool {
        // All mandatory jobs assigned 
        if requires_completeness && (!self.assigned_maj_maint.all() || !self.assigned_tasks.all() && !self.uncovered.iter().all(|x| x.is_empty())) { return false; }

        // Correct maint assignments
        for (i, time) in self.maj_maint_ends.iter().enumerate() {
            if self.jobs[i].get(time) != Some(&JobToken::MajMaint) {
                return false;
            }
        }
        // Maj maint overlaps
        for i in 0..self.instance.resources() {
            for j in i+1..self.instance.resources() {
                if self.maj_maint_ends[i].abs_diff(self.maj_maint_ends[j]) < self.instance.duration_major() { return false; }
            }
        }
        // No overlap + maint coverage + maj uniqueness + reg matching
        let mut tasks = self.assigned_tasks.clone();
        let mut previous = 0;
        tasks.negate();
        for (res, jobs) in self.jobs.iter().enumerate() {
            for (time, job) in jobs.iter() {
                let difference = *time - previous;
                match job {
                    // Check maint assignments
                    JobToken::MajMaint => {
                        if *time != self.maj_maint_ends[res] || difference < self.instance.duration_major() { return false; }
                    },
                    JobToken::RegMaint => { 
                        if !self.reg_maint_ends[res].contains(time) || difference < self.instance.duration_regular() { return false; }
                    },
                    JobToken::Task(i) => {
                        if tasks[*i] || difference < self.instance.tasks()[*i].length() { return false; }   // Double assignment or unassigned occurring
                        tasks.set(*i, true);

                        // Check coverage + uncovered assignment
                        if *time > self.instance.time_regular() && self.has_maint_covered(res, *time).is_none() {
                            if requires_completeness || !self.uncovered[res].contains(time) { return false; }
                        }
                    }
                }
                previous = *time;
            }
            // Check reg maint assignments
            if self.reg_maint_ends[res].len() != self.num_reg_maint[res] { return false; }
        }

        true
    }

    pub fn mm_overlaps_with_other_mm(&self, resource: usize, end_time: usize) -> bool {
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
        self.task_ass[task_idx] = resource;
        self.jobs[resource].insert(self.instance.tasks()[task_idx].end(), JobToken::Task(task_idx));
    }

    pub fn remove_task(&mut self, resource: usize, task_idx: usize) {
        self.assigned_tasks.set(task_idx, false);
        self.task_ass[task_idx] = usize::MAX;
        let end_time = self.instance.tasks()[task_idx].end();
        self.jobs[resource].remove(&end_time);
        if self.uncovered[resource].contains(&end_time) {
            self.uncovered[resource].remove(&end_time);
            let last_maint = self.jobs[resource].range(..end_time)
                .filter(|(_, job)| **job == JobToken::MajMaint || **job == JobToken::RegMaint).next_back();
            let task = &self.instance.tasks()[task_idx];
            let previously_uncovered = if last_maint.is_none() || task.start() > *last_maint.unwrap().0 {
                task.length()
            } else {
                last_maint.unwrap().0 - task.start()
            };
            // println!("Remove task {}", PenaltyToken::RegMaintNotCovered(previously_uncovered).to_penalty(&self.instance, self.penalty_multi));
            self.penalty_value -= PenaltyToken::RegMaintNotCovered(previously_uncovered).to_penalty(&self.instance, self.penalty_multi);
            // Task was uncovered, remove penalty for it
        }
        // println!("Add task {}", PenaltyToken::Task(task_idx).to_penalty(&self.instance, self.penalty_multi));
        self.penalty_value += PenaltyToken::Task(task_idx).to_penalty(&self.instance, self.penalty_multi);
    }

    pub fn add_major_maintenence(&mut self, res: usize, end_time: usize) {
        let start_time = end_time - self.instance.duration_major();
        self.assigned_maj_maint.set(res, true);
        self.maj_maint_ends[res] = end_time;
        self.jobs[res].insert(end_time, JobToken::MajMaint);
        
        self.update_changes_maint_added(start_time, end_time, res);
    }

    pub fn repair_after_move(&mut self, res: usize, prev_time: usize, new_time: usize) {
        // Repair if a task was (partly) uncovered from move, but only if it was completely covered before
        if new_time >= prev_time { return; }    // Not possible to uncover if moving forward
        // println!("try repair!");
        let new_uncovered = self.uncovered[res].range(new_time+self.instance.time_regular()..prev_time+self.instance.time_regular()+1).next_back();
        if new_uncovered.is_none() { return; }  // No task uncovered

        // Cover the task
        // println!("Cover!");
        // println!("{:?}", self.uncovered);
        if let Some(new_rm) = self.find_reg_maint_cover_random(res, *new_uncovered.unwrap()) {
            self.add_regular_maintenance(res, new_rm);
        }
    }

    pub fn remove_major_maintenence(&mut self, res: usize) {
        let end_time = self.maj_maint_ends[res];
        let start_time = end_time - self.instance.duration_major();
        self.assigned_maj_maint.set(res, false);
        self.maj_maint_ends[res] = 0;
        self.jobs[res].remove(&end_time);

        self.update_changes_maint_removed(start_time, end_time, res);
    }

    pub fn add_regular_maintenance(&mut self, res: usize, end_time: usize) {
        let start_time = end_time - self.instance.duration_regular();
        self.reg_maint_ends[res].insert(end_time);
        self.num_reg_maint[res] += 1;
        self.jobs[res].insert(end_time, JobToken::RegMaint);

        self.update_changes_maint_added(start_time, end_time, res);
    }

    pub fn remove_regular_maintenance(&mut self, res: usize, end_time: usize) {
        let start_time = end_time - self.instance.duration_regular();
        self.reg_maint_ends[res].remove(&end_time);
        self.num_reg_maint[res] -= 1;
        self.jobs[res].remove(&end_time);

        self.update_changes_maint_removed(start_time, end_time, res);
    }

    // Calculate upper and lower bounds for end time (possibly to freely move maintenance between these two timeframes)
    pub fn get_neighbors(&self, res: usize, time: usize, is_mm: bool) -> (usize, usize) {
        let len = if is_mm { self.instance.duration_major() } else { self.instance.duration_regular() };
        let mut left = match self.jobs[res].range(..time).next_back() {
            Some((x, _)) => x + len,
            None => len
        };
        let mut right = match self.jobs[res].range(time+1..).next() {
            Some((x, job)) => x - match job {
                JobToken::MajMaint => self.instance.duration_major(),
                JobToken::RegMaint => self.instance.duration_regular(),
                JobToken::Task(i) => self.instance.tasks()[*i].length()
            },
            None => self.instance.horizon()
        };
        if is_mm {
            for other_res in 0..self.instance.resources() {
                if other_res == res { continue; }
                let other = self.maj_maint_ends[other_res];
                if other < time && other + self.instance.duration_major() > left {
                    left = other + self.instance.duration_major();
                } else if other > time && other - self.instance.duration_major() < right {
                    right = other - self.instance.duration_major();
                }
            }
        }
        (left, right)
    }


    // If the task is covered by a maintenance, returns Some(maint time), where maint time is the end time of the closest maint one that coveres the task
    fn has_maint_covered(&self, res: usize, time: usize) -> Option<usize> {
        let limit = time - self.instance.time_regular();
        match self.jobs[res].range(limit..time).find(|x| x.1 == &JobToken::MajMaint || x.1 == &JobToken::RegMaint) {
            Some(x) => Some(*x.0),
            None => None
        }
    }

    // Add reg maintenance greedily at first suitable position
    pub fn find_reg_maint_cover_greedy(&self, res: usize, time: usize) -> Option<usize> {
        let mut possible_start = cmp::max(time as isize - self.instance.time_regular() as isize - self.instance.duration_regular() as isize, 0) as usize;
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
            return Some(possible_start + self.instance.duration_regular())
        }

        None
    }

    // Add reg maintenance a random (but covering) position
    pub fn find_reg_maint_cover_random(&self, res: usize, time: usize) -> Option<usize> {
        // TODO: implement
        let mut possible_start = cmp::max(time as isize - self.instance.time_regular() as isize - self.instance.duration_regular() as isize, 0) as usize;
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
            return Some(possible_start + self.instance.duration_regular())
        }

        // None

        unimplemented!()
    }

    // (res, time)
    pub fn get_rand_rm(&self) -> Option<(usize, usize)> {
        let mut rng = thread_rng();
        if self.reg_maint_ends.iter().all(|x| x.is_empty()) {
            return None;
        }
        // TODO: Improve: select from all equally weighted instead of loop
        loop {
            let res = rng.gen_range(0..self.instance.resources());
            if self.reg_maint_ends[res].is_empty() { continue; }

            return Some((res, *self.reg_maint_ends[res].iter().skip(rng.gen_range(0..self.reg_maint_ends[res].len())).next().unwrap()))
        }
    }

    // res
    pub fn get_rand_mm(&self) -> usize {
        let mut rng = thread_rng();
        rng.gen_range(0..self.instance.resources())
    }

    // taskid
    pub fn get_rand_task(&self) -> usize {
        let mut rng = thread_rng();
        rng.gen_range(0..self.instance.tasks().len())
    }

    // Updates objective values, maintenance changes, uncovered and penalty when a maintenance is added
    fn update_changes_maint_added(&mut self, start_time: usize, end_time: usize, res: usize) {
        // Update maintenance changes
        let num_before_start = match self.maintenance_changes.range(..start_time).next_back() {
            Some(x) => x.1.count,
            None => 0
        };        
        let num_before_end = match self.maintenance_changes.range(..end_time).next_back() {
            Some(x) => x.1.count,
            None => 0
        };
        let start = self.maintenance_changes.entry(start_time).or_insert(ChangeTimestamp::new(0, num_before_start));
        start.num_changed += 1;
        let end = self.maintenance_changes.entry(end_time).or_insert(ChangeTimestamp::new(0, num_before_end));
        end.num_changed += 1;
        for (_, stamp) in self.maintenance_changes.range_mut(start_time..end_time) {
            stamp.count += 1;
        }
        // Update obj value
        let mut prev = (start_time, *self.maintenance_changes.get(&start_time).unwrap());
        let mut change = 0;
        for (&curr, stamp) in self.maintenance_changes.range(start_time+1..end_time+1) {
            let dist = curr - prev.0;
            change += (prev.1.count * prev.1.count - (prev.1.count - 1) * (prev.1.count - 1)) * dist;
            prev = (curr, *stamp)
        }
        self.obj_value += change;

        // Update uncovered and penalties
        // Compute all tasks that are uncovered and overlap with cover limit of new maintenance
        let cover_limit = end_time + self.instance.time_regular();
        let mut affected_tasks = Vec::new();
        // println!("{:?}", self.uncovered);
        for time in self.uncovered[res].range(end_time..) {
            // println!("Update task {}", *time);
            let task = self.jobs[res].get(time).unwrap();
            match task {
                JobToken::Task(id) => {
                    // println!("Add task {}", *time);
                    affected_tasks.push((*time, *id)); 
                },
                _ => panic!("Maintenance assigned to task???")
            }
            if *time > cover_limit {
                // Break after first uncovered task that exceeds cover limit
                break; 
            }
        }
        // Update all tasks that were uncovered and are effected
        let prev_maint_limit = match self.jobs[res].range(..end_time)
            .filter(|(_, job)| **job == JobToken::MajMaint || **job == JobToken::RegMaint).next_back() 
        {
            Some((t, _)) => *t,
            None => 0
        } + self.instance.time_regular();
        for (time, task_id) in affected_tasks.iter() {
            let task = &self.instance.tasks()[*task_id];
            let covered_by_prev = if task.start() >= prev_maint_limit {
                0
            } else {
                prev_maint_limit - task.start()
            };
            let newly_covered_time = if cover_limit >= task.end() { 
                task.length()
            } else {
                cover_limit - task.start()
            } - covered_by_prev;
            
            // println!("Remove {}", PenaltyToken::RegMaintNotCovered(newly_covered_time).to_penalty(&self.instance, self.penalty_multi));
            self.penalty_value -= PenaltyToken::RegMaintNotCovered(newly_covered_time).to_penalty(&self.instance, self.penalty_multi);
            if *time <= cover_limit {
                self.uncovered[res].remove(time);
            }
        }
        // TODO: Check all corner cases for uncovered
    }

    // Updates objective values, maintenance changes, uncovered and penalty when a maintenance is removed
    fn update_changes_maint_removed(&mut self, start_time: usize, end_time: usize, res: usize) {
        // Update maintenance changes
        for (_, stamp) in self.maintenance_changes.range_mut(start_time..end_time) {
            stamp.count -= 1;
        }
        self.maintenance_changes.get_mut(&start_time).unwrap().num_changed -= 1;
        self.maintenance_changes.get_mut(&end_time).unwrap().num_changed -= 1;
        // TODO: Can get None?

        // Update obj valuee
        let mut prev = (start_time, *self.maintenance_changes.get(&start_time).unwrap());
        let mut change = 0;
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

        // Update uncovered and penalties
        // Compute all tasks that might now be uncovered and overlap with cover limit of new maintenance
        let cover_limit = end_time + self.instance.time_regular();
        let mut affected_tasks = Vec::new();
        for (time, job) in self.jobs[res].range(end_time..) {
            if self.has_maint_covered(res, *time).is_some() { continue; }   // Is covered by another maintenance
            match job {
                JobToken::Task(id) => { affected_tasks.push((*time, *id)); },
                _ => break  // Anything after this maintenance wasn't covered by the removed one
            }
            if *time > cover_limit {
                // Break after first uncovered task that exceeds cover limit
                break; 
            }
        }
        // Update all tasks that were uncovered and are effected
        let prev_maint_limit = match self.jobs[res].range(..end_time)
            .filter(|(_, job)| **job == JobToken::MajMaint || **job == JobToken::RegMaint).next_back() 
        {
            Some((t, _)) => *t,
            None => 0
        } + self.instance.time_regular();
        for (time, task_id) in affected_tasks.iter() {
            let task = &self.instance.tasks()[*task_id];
            let covered_by_prev = if task.start() >= prev_maint_limit {
                0
            } else {
                prev_maint_limit - task.start()
            };
            let previously_covered_time = if cover_limit >= task.end() { 
                task.length()
            } else {
                cover_limit - task.start()
            } - covered_by_prev;
            // println!("Add {}", PenaltyToken::RegMaintNotCovered(previously_covered_time).to_penalty(&self.instance, self.penalty_multi));
            self.penalty_value += PenaltyToken::RegMaintNotCovered(previously_covered_time).to_penalty(&self.instance, self.penalty_multi);
            if *time <= cover_limit {
                self.uncovered[res].insert(*time);
            }
        }
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