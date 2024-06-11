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
        // Add initial penalties for unassigned stuff
        for _ in 0..self.instance.resources() {
            self.penalty_value += PenaltyToken::MajMaint.to_penalty(&self.instance, self.penalty_multi);
        }
        for task_id in 0..self.instance.tasks().len() {
            self.penalty_value += PenaltyToken::Task(task_id).to_penalty(&self.instance, self.penalty_multi);
        }
        // Add major maintenances at random (non-overlapping) times
        for res in 0..self.instance.resources() {
            self.add_major_maintenance(res, (res+1) * self.instance.duration_major());
        }
        // Assign tasks to the first (free) resource
        for task_id in 0..self.instance.tasks().len() {
            if let Some(res) = (0..self.instance.resources()).find(|res| self.can_add_task(*res, task_id)) {
                self.add_task(res, task_id);
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
        }
    }

    pub fn is_feasible_quick(&self) -> bool {
        self.penalty_value == 0
    }

    pub fn increase_penalty_multi(&mut self) {
        let old_multi = self.penalty_multi;
        self.penalty_multi += 1;
        self.penalty_value /= old_multi;
        self.penalty_value *= self.penalty_multi;
    }

    pub fn calc_penalty_from_scratch(&self) -> usize {
        let mut penalty = 0;
        // Uncovered tasks
        for res in 0..self.instance.resources() {
            let mut cover_limit = self.instance.time_regular();
            for (time, job) in self.jobs[res].iter() {
                match job {
                    JobToken::Task(id) => {
                        if *time <= cover_limit {
                            // Covered
                            continue;
                        }
                        let task = &self.instance.tasks()[*id];
                        penalty += PenaltyToken::RegMaintNotCovered(if task.start() > cover_limit {
                            // Fully uncovered
                            task.length()
                        } else {
                            // Partially covered
                            task.end() - cover_limit
                        }).to_penalty(&self.instance, self.penalty_multi);
                    },
                    _ => cover_limit = *time + self.instance.time_regular()
                };
            }
        }

        // Unassigned tasks
        for (task_id, _) in self.assigned_tasks.iter().enumerate().filter(|(_, b)| !*b) {
            penalty += PenaltyToken::Task(task_id).to_penalty(&self.instance, self.penalty_multi);
        }
        
        // Unassigned maj maintenances
        for _ in self.assigned_maj_maint.iter().enumerate().filter(|(_, b)| !*b) {
            penalty += PenaltyToken::MajMaint.to_penalty(&self.instance, self.penalty_multi);
        }

        penalty
    }

    pub fn is_feasible(&self, requires_completeness: bool) -> bool {
        // All mandatory jobs assigned 
        if requires_completeness && (!self.assigned_maj_maint.all() || !self.assigned_tasks.all() && !self.uncovered.iter().all(|x| x.is_empty())) {
            eprintln!("Not all jobs assigned, but mandatory enabled");
            return false; 
        }

        // Correct maint assignments
        for (i, time) in self.maj_maint_ends.iter().enumerate() {
            if self.jobs[i].get(time) != Some(&JobToken::MajMaint) {
                eprintln!("maj maint incorrect assignment");
                return false;
            }
        }
        // Maj maint overlaps
        for i in 0..self.instance.resources() {
            for j in i+1..self.instance.resources() {
                if self.maj_maint_ends[i].abs_diff(self.maj_maint_ends[j]) < self.instance.duration_major() {
                    eprintln!("maj maint overlap");
                    return false; 
                }
            }
        }
        // No overlap + maint coverage + maj uniqueness + reg matching
        let mut tasks = self.assigned_tasks.clone();
        tasks.negate();
        for (res, jobs) in self.jobs.iter().enumerate() {
            let mut previous = 0;
            // eprintln!("Check res {}", res);
            for (time, job) in jobs.iter() {
                // eprintln!("Check time {}", time);
                let difference = *time - previous;
                match job {
                    // Check maint assignments
                    JobToken::MajMaint => {
                        if *time != self.maj_maint_ends[res] || difference < self.instance.duration_major() {
                            eprintln!("{}={}", time, self.maj_maint_ends[res]);
                            eprintln!("{}<{}, prev: {}", difference, self.instance.duration_major(), previous);
                            eprintln!("maj maint incorrect assignment 2");
                            return false; 
                        }
                    },
                    JobToken::RegMaint => { 
                        if !self.reg_maint_ends[res].contains(time) || difference < self.instance.duration_regular() {
                            eprintln!("reg maint incorrect assignment in res {}, time {}, difference: {}", res, time, difference); 
                            return false; 
                        }
                    },
                    JobToken::Task(i) => {
                        if tasks[*i] || difference < self.instance.tasks()[*i].length() {
                            eprintln!("Double assignment or unassigned occurring");
                            return false; 
                        }   
                        tasks.set(*i, true);

                        // Check coverage + uncovered assignment
                        if *time > self.instance.time_regular() && self.has_maint_covered(res, *time).is_none() {
                            if requires_completeness || !self.uncovered[res].contains(time) {
                                eprintln!("error in coverage and uncovered assignments");
                                return false; 
                            }
                        }
                    }
                }
                previous = *time;
            }
        }

        true
    }

    pub fn mm_overlaps_with_other_mm(&self, resource: usize, end_time: usize) -> bool {
        self.maj_maint_ends.iter().enumerate().any(|(res, end)| res != resource && (*end as isize - end_time as isize).abs() < self.instance.duration_major() as isize)
    }

    pub fn can_add_task(&self, resource: usize, task_id: usize) -> bool {
        let start = self.instance.tasks()[task_id].start();
        let end = self.instance.tasks()[task_id].end();
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

    pub fn add_task(&mut self, res: usize, task_id: usize) {
        self.assigned_tasks.set(task_id, true);
        self.task_ass[task_id] = res;
        let task = &self.instance.tasks()[task_id];
        self.jobs[res].insert(task.end(), JobToken::Task(task_id));

        // Update penalties
        self.penalty_value -= PenaltyToken::Task(task_id).to_penalty(&self.instance, self.penalty_multi);
        // Uncovered penalties
        if task.end() <= self.instance.time_regular(){ return; }    // All covered in first timeframe

        let cover_limit = match self.jobs[res].range(..task.start()+1).filter(|(_, job)| **job == JobToken::MajMaint || **job == JobToken::RegMaint).next_back() {
            Some((time, _)) => *time,
            None => 0
        } + self.instance.time_regular();

        if cover_limit >= task.end() { return; }    // All covered
        let additional_penalty = cmp::min(task.end() - cover_limit, task.length());
        self.uncovered[res].insert(task.end());
        self.penalty_value += PenaltyToken::RegMaintNotCovered(additional_penalty).to_penalty(&self.instance, self.penalty_multi);
    }

    pub fn remove_task(&mut self, task_id: usize) {
        let res = self.task_ass[task_id];
        self.assigned_tasks.set(task_id, false);
        self.task_ass[task_id] = usize::MAX;
        let end_time = self.instance.tasks()[task_id].end();
        // println!("end: {}, uncovered: {}", end_time, self.uncovered[res].contains(&end_time));
        self.jobs[res].remove(&end_time);
        
        // Update penalties
        if self.uncovered[res].contains(&end_time) {
            self.uncovered[res].remove(&end_time);
            let cover_limit = match self.jobs[res].range(..end_time).filter(|(_, job)| **job == JobToken::MajMaint || **job == JobToken::RegMaint).next_back() {
                    Some((time, _)) => *time,
                    None => 0
            } + self.instance.time_regular();
            let task = &self.instance.tasks()[task_id];
            // println!("limit = {:?}", cover_limit);
            let previously_uncovered = if task.start() > cover_limit {
                // println!("length");
                task.length()
            } else {
                // println!("calc");
                task.end() - cover_limit
            };
            // println!("Prev: {}", previously_uncovered);
            // println!("Remove {}", PenaltyToken::RegMaintNotCovered(previously_uncovered).to_penalty(&self.instance, self.penalty_multi));
            self.penalty_value -= PenaltyToken::RegMaintNotCovered(previously_uncovered).to_penalty(&self.instance, self.penalty_multi);
            // Task was uncovered, remove penalty for it
        }
        // println!("Add {}", PenaltyToken::Task(task_id).to_penalty(&self.instance, self.penalty_multi));
        self.penalty_value += PenaltyToken::Task(task_id).to_penalty(&self.instance, self.penalty_multi);
    }

    pub fn add_major_maintenance(&mut self, res: usize, end_time: usize) {
        let start_time = end_time - self.instance.duration_major();
        self.assigned_maj_maint.set(res, true);
        self.maj_maint_ends[res] = end_time;
        self.jobs[res].insert(end_time, JobToken::MajMaint);
        
        self.update_changes_maint_added(start_time, end_time, res);
        self.penalty_value -= PenaltyToken::MajMaint.to_penalty(&self.instance, self.penalty_multi);
    }

    pub fn remove_major_maintenance(&mut self, res: usize) {
        let end_time = self.maj_maint_ends[res];
        let start_time = end_time - self.instance.duration_major();
        self.assigned_maj_maint.set(res, false);
        self.maj_maint_ends[res] = 0;
        self.jobs[res].remove(&end_time);

        self.update_changes_maint_removed(start_time, end_time, res);

        self.penalty_value += PenaltyToken::MajMaint.to_penalty(&self.instance, self.penalty_multi);
    }

    pub fn add_regular_maintenance(&mut self, res: usize, end_time: usize) {
        let start_time = end_time - self.instance.duration_regular();
        self.reg_maint_ends[res].insert(end_time);
        self.jobs[res].insert(end_time, JobToken::RegMaint);

        self.update_changes_maint_added(start_time, end_time, res);
    }

    pub fn remove_regular_maintenance(&mut self, res: usize, end_time: usize) {
        let start_time = end_time - self.instance.duration_regular();
        self.reg_maint_ends[res].remove(&end_time);
        self.jobs[res].remove(&end_time);

        self.update_changes_maint_removed(start_time, end_time, res);
    }
    
    // TODO: Consider adding previously partially covered as well (increase range)
    pub fn repair_after_move(&mut self, res: usize, prev_time: usize, new_time: usize) -> Option<usize> {
        // Repair if a task was (partly) uncovered from move, but only if it was completely covered before
        if new_time >= prev_time { return None; }    // Not possible to uncover if moving forward
        // println!("try repair!");
        let new_uncovered = self.uncovered[res].range(new_time+self.instance.time_regular()..prev_time+self.instance.time_regular()+1).next_back();
        if new_uncovered.is_none() { return None; }  // No task uncovered

        // Cover the task
        // println!("Cover!");
        // println!("{:?}", self.uncovered);
        
        match self.find_reg_maint_cover_random(res, *new_uncovered.unwrap()) {
            Some(new_rm) => {
                self.add_regular_maintenance(res, new_rm);
                return Some(new_rm);
            },
            None => return None
        };
    }

    // TODO: Consider adding previously partially covered as well (increase range)
    pub fn repair_after_move_any(&mut self, res: usize, prev_time: usize) -> Option<usize> {
        // Repair if a task was (partly) uncovered from move, but only if it was completely covered before
        // println!("try repair!");
        let new_uncovered = self.uncovered[res].range(prev_time..prev_time+self.instance.time_regular()+1).next_back();
        if new_uncovered.is_none() { return None; }  // No task uncovered

        // Cover the task
        // println!("Cover!");
        // println!("{:?}", self.uncovered);
        
        match self.find_reg_maint_cover_random(res, *new_uncovered.unwrap()) {
            Some(new_rm) => {
                self.add_regular_maintenance(res, new_rm);
                return Some(new_rm);
            },
            None => return None
        };
    }

    // TODO: Consider adding previously partially covered as well (increase range)
    pub fn repair_after_remove(&mut self, res: usize, prev_time: usize) -> Option<usize> {
        // Repair if a task was (partly) uncovered from removal, but only if it was completely covered before
        // println!("try repair!");
        let new_uncovered = self.uncovered[res].range(prev_time..prev_time+self.instance.time_regular()+1).next_back();
        if new_uncovered.is_none() { return None; }  // No task uncovered

        // Cover the task
        // println!("Cover!");
        // println!("{:?}", self.uncovered);
        
        match self.find_reg_maint_cover_random(res, *new_uncovered.unwrap()) {
            Some(new_rm) => {
                self.add_regular_maintenance(res, new_rm);
                return Some(new_rm);
            },
            None => return None
        };
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
        if time <= self.instance.time_regular() {
            return Some(0);
        }
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
        let first_possible_end = cmp::max(time as isize - self.instance.time_regular() as isize, self.instance.duration_regular() as isize) as usize;
        // println!("window: {}-{}", first_possible_end, time);
        let windows = self.get_all_suitable_windows_on_res(res, first_possible_end, time, self.instance.duration_regular(), false);
        if windows.is_empty() {
            return None;
        }
        let mut rng = thread_rng();
        // println!("{:?}", windows);
        let (left, right) = windows.choose(&mut rng).unwrap();
        let selected = rng.gen_range(*left..*right+1);
        // println!("res {}: {}/{} => {}", res, left, right, selected);
        // println!("{:?}", self);
        
        Some(selected)
    }

    // Window to add job without need to remove anything
    pub fn get_all_suitable_windows_on_res(&self, res: usize, window_start: usize, window_end: usize, length: usize, is_mm: bool) -> Vec<(usize, usize)> {
        if window_start < length || (is_mm && length != self.instance.duration_major()) {
            panic!("Requires end times => window_start >= length, or maj maint duration not correct")
        }
        let mut possible_windows = Vec::new();
        let mut prev = window_start - length;
        let mut finished_interval = false;
        for (end, job) in self.jobs[res].range(window_start - length..) {
            if *end < window_start - length { continue; }    // No need to check jobs that would overlap if end is set to < window_start-length
            let start = match job {
                JobToken::MajMaint => *end - self.instance.duration_major(),
                JobToken::RegMaint => *end - self.instance.duration_regular(),
                JobToken::Task(id) => self.instance.tasks()[*id].start()
            };
            // println!("res={}, start={}, prev={}, add={}", res, start, prev, start >= prev && start - prev >= length);
            if start >= prev && start - prev >= length {    // start can be < prev if prev initialized to length
                // Suitable window found
                possible_windows.push((prev+length, start));
            }
            prev = *end;
            if prev >= window_end {
                finished_interval = true;
                break;
            }
        }
        if !finished_interval {
            // Need to check window after last job until window_end
            let last_job_end = match self.jobs[res].last_key_value() {
                Some((i, _)) => *i,
                None => length
            };
            if window_end - last_job_end >= length {
                possible_windows.push((last_job_end + length, window_end));
            }
        }

        if !is_mm { return possible_windows; }

        // println!("possible for res {}: {:?}", res, possible_windows);

        // Check overlaps of MMs
        let mut windows_for_mm = Vec::new();
        for window in possible_windows.into_iter() {
            let mut splits = vec![window];
            while !splits.is_empty() {
                let (mut left, mut right) = splits.pop().unwrap();
                // println!("left={}, right={}", left, right);
                let mut can_add = true;
                for other_res in 0..self.instance.resources() {
                    if other_res == res {
                        continue;   // Skip self
                    }
                    let end = self.maj_maint_ends[other_res];
                    // println!("res {}: end={}", other_res, end);
                    if end <= left - length || end >= right + length {
                        // println!("No overlap");
                        continue;   // Not overlapping with window
                    }
                    // Overlap, case cannot fit MM between left and start of other_mm
                    // println!("first: {}", end < left + 2 * length);
                    if end < left + 2 * length {
                        left = end + length;
                        continue;
                    }
                    // println!("left={}, right={}", left, right);
                    // Overlap, case cannot fit MM between right and end of other_mm
                    // println!("second: {}", end > right - length);
                    if end > right - length {
                        right = end - length;
                        continue;
                    }
                    // println!("left={}, right={}", left, right);
                    // println!("third: {}", left > right || right - left < length);
                    if left > right {
                        can_add = false;
                        break;  // Cannot fit MM in window
                    }
                    // println!("left={}, right={}", left, right);
                    // Case that it is in the middle and we can fit a MM either to the left or to the right => need to split
                    let leftwindow = (left, end-length);
                    let rightwindow = (end, right);
                    if leftwindow.0 <= leftwindow.1 {
                        // println!("Add {:?}", leftwindow);
                        splits.push(leftwindow);
                    }
                    if rightwindow.0 <= rightwindow.1 {
                        // println!("Add {:?}", rightwindow);
                        splits.push(rightwindow);
                    }
                    can_add = false;
                    break;
                }
                
                if can_add && right > left {
                    windows_for_mm.push((left, right));
                }
            }
        }

        windows_for_mm
    }

    // (res, time)
    pub fn get_rand_rm(&self) -> Option<(usize, usize)> {
        let num_rm = self.reg_maint_ends.iter().flatten().count();
        if num_rm == 0 {
            return None;
        }
        let rm_idx = thread_rng().gen_range(0..num_rm);
        let mut counter = 0;
        for res in 0..self.instance.resources() {
            for rm in self.reg_maint_ends[res].iter() {
                if counter == rm_idx {
                    return Some((res, *rm));
                }
                counter += 1;
            }
        }

        panic!("Should always find the index")
    }

    // (res, time)
    pub fn get_rand_mm(&self) -> Option<(usize, usize)> {
        let num_assigned = self.assigned_maj_maint.iter().filter(|b| *b).count();
        if num_assigned == 0 {
            return None;
        }
        let res = self.assigned_maj_maint.iter().enumerate().filter(|(_, b)| *b).skip(thread_rng().gen_range(0..num_assigned)).next().unwrap().0;
        Some((res, self.maj_maint_ends[res]))
    }    

    // (res, time)
    pub fn get_rand_unassigned_mm(&self) -> Option<(usize, usize)> {
        let num_unassigned = self.assigned_maj_maint.iter().filter(|b| !*b).count();
        if num_unassigned == 0 {
            return None;
        }
        let res = self.assigned_maj_maint.iter().enumerate().filter(|(_, b)| !*b).skip(thread_rng().gen_range(0..num_unassigned)).next().unwrap().0;
        Some((res, self.maj_maint_ends[res]))
    }

    // taskid
    pub fn get_rand_unassigned_task(&self) -> Option<usize> {
        let num_unassigned = self.assigned_tasks.iter().filter(|b| !*b).count();
        if num_unassigned == 0 {
            return None;
        }
        
        Some(self.assigned_tasks.iter().enumerate().filter(|(_, b)| !*b).skip(thread_rng().gen_range(0..num_unassigned)).next().unwrap().0)
    }

    // (res, taskid)
    pub fn get_rand_assigned_task(&self) -> Option<(usize, usize)> {
        let num_assigned = self.assigned_tasks.iter().filter(|b| *b).count();
        if num_assigned == 0 {
            return None;
        }
        let task_id = self.assigned_tasks.iter().enumerate().filter(|(_, b)| *b).skip(thread_rng().gen_range(0..num_assigned)).next().unwrap().0;
        Some((self.task_ass[task_id], task_id))
    }

    // (res, time)
    pub fn get_rand_uncovered_task(&self) -> Option<(usize, usize)> {
        let num_uncovered = self.uncovered.iter().flatten().count();
        if num_uncovered == 0 {
            return None;
        }
        let idx = thread_rng().gen_range(0..num_uncovered);
        let mut counter = 0;
        for res in 0..self.instance.resources() {
            for time in self.uncovered[res].iter() {
                if counter == idx {
                    return Some((res, *time));
                }
                counter += 1;
            }
        }

        panic!("Should always find the index")
    }

    // Updates objective values, maintenance changes, uncovered and penalty when a maintenance is added
    fn update_changes_maint_added(&mut self, start_time: usize, end_time: usize, res: usize) {
        // println!("------------------------Add Maint-------------------------");
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
        // println!("coverlimit = {}", cover_limit);
        let mut affected_tasks = Vec::new();
        // println!("uncovered: {:?}", self.uncovered);
        for (time, job) in self.jobs[res].range(end_time+1..) {
            match job {
                JobToken::Task(id) => {
                    // println!("check task {} ({})", id, time);
                    if !self.uncovered[res].contains(time) {
                        // Task covered
                        continue;
                    }
                    if self.instance.tasks()[*id].start() < cover_limit {
                        // println!("Add task {}", *time);
                        affected_tasks.push((*time, *id)); 
                    } else {
                        // Break after first uncovered task that exceeds cover limit
                        break;
                    }
                },
                _ => {
                    // Maintenance => stop, rest is covered
                    break;
                }
            }
        }
        // Update all tasks that were uncovered and are effected
        let prev_maint_limit = match self.jobs[res].range(..end_time)
            .filter(|(_, job)| **job == JobToken::MajMaint || **job == JobToken::RegMaint).next_back() 
        {
            Some((t, _)) => *t,
            None => 0
        } + self.instance.time_regular();
        // println!("prev_limit: {}", prev_maint_limit);
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
            
            // println!("Remove {} for task {}({})", PenaltyToken::RegMaintNotCovered(newly_covered_time).to_penalty(&self.instance, self.penalty_multi), task_id, time);
            self.penalty_value -= PenaltyToken::RegMaintNotCovered(newly_covered_time).to_penalty(&self.instance, self.penalty_multi);
            if *time <= cover_limit {
                self.uncovered[res].remove(time);
            }
        }
    }

    // Updates objective values, maintenance changes, uncovered and penalty when a maintenance is removed
    fn update_changes_maint_removed(&mut self, start_time: usize, end_time: usize, res: usize) {
        // println!("---------------------Remove Maint-------------------------");
        // Update maintenance changes
        for (_, stamp) in self.maintenance_changes.range_mut(start_time..end_time) {
            stamp.count -= 1;
        }
        // println!("removed at: start {}, end {}", start_time, end_time);
        self.maintenance_changes.get_mut(&start_time).unwrap().num_changed -= 1;
        self.maintenance_changes.get_mut(&end_time).unwrap().num_changed -= 1;

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
        // println!("coverlimit={}", cover_limit);
        let mut affected_tasks = Vec::new();
        for (time, job) in self.jobs[res].range(end_time..) {
            // println!("time={}", time);
            match job {
                JobToken::Task(id) => {
                    if self.has_maint_covered(res, *time).is_some() { continue; }   // Is covered by another maintenance
                    if self.instance.tasks()[*id].start() < cover_limit {
                        // println!("Add job {} ({})", id, time);
                        affected_tasks.push((*time, *id)); 
                    } else {
                        // Break after first uncovered task that exceeds cover limit
                        break;
                    }
                },
                _ => {
                    // println!("Term due to maint at {}", time);
                    break; 
                }  // Anything after this maintenance wasn't covered by the removed one
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