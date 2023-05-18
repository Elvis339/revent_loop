use std::collections::VecDeque;
use std::ops::{Sub};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fmt, thread};
use uuid::Uuid;

struct Task {
    id: Uuid,
    callback: Box<dyn FnOnce() + Send + 'static>,
    expires: Option<Duration>,
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task").field("id", &self.id).finish()
    }
}

impl Task {
    fn new(callback: impl FnOnce() + Send + 'static, expires: Option<Duration>) -> Self {
        Self {
            id: Uuid::new_v4(),
            callback: Box::new(callback),
            expires,
        }
    }
}

struct Scheduler {
    ready_fns: Mutex<VecDeque<Task>>,
    sleeping_fns: Mutex<VecDeque<Task>>,
}

impl Scheduler {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            ready_fns: Mutex::new(VecDeque::new()),
            sleeping_fns: Mutex::new(VecDeque::new()),
        })
    }

    fn schedule(&self, mut task: Task) {
        match task.expires {
            None => {
                let mut ready_fns_guard = self.ready_fns.lock().unwrap();
                ready_fns_guard.push_back(task);
                drop(ready_fns_guard);
            }
            Some(expires) =>{
                let mut sleeping_fns_guard = self.sleeping_fns.lock().unwrap();
                task.expires = Some(expires);

                // @todo: sort before pushing
                sleeping_fns_guard.push_back(task);
                drop(sleeping_fns_guard);
            }
        }
    }

    fn run(&self) {
        let is_empty = |task: &str| {
            if task == "ready" {
                let ready_guard = self.ready_fns.lock().unwrap();
                let empty = ready_guard.is_empty();
                drop(ready_guard);
                empty
            } else {
                let sleep_guard = self.sleeping_fns.lock().unwrap();
                let empty = sleep_guard.is_empty();
                drop(sleep_guard);
                empty
            }
        };

        let run_sleeping = || {
            let mut sleeping_tasks = self.sleeping_fns.lock().unwrap();
            if let Some(task) = sleeping_tasks.pop_front() {
                if let Some(_) = task.expires {
                    let now = Instant::now();
                    let delta = task.expires.unwrap().sub(now.elapsed());
                    if delta.as_secs() > 0 {
                        thread::sleep(delta);
                    }
                    let mut ready_tasks = self.ready_fns.lock().unwrap();
                    ready_tasks.push_back(task);
                    drop(ready_tasks);
                }
            }
            drop(sleeping_tasks);
        };

        let run_active = || {
            let mut ready_task = self.ready_fns.lock().unwrap();
            while let Some(task) = ready_task.pop_front() {
                drop(ready_task);
                (task.callback)();
                ready_task = self.ready_fns.lock().unwrap();
            }
        };

        while !is_empty("ready") || !is_empty("sleep") {
            if is_empty("ready") {
                run_sleeping();
            }
            run_active();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn countdown(n: usize, scheduler: Arc<Scheduler>) {
        if n > 0 {
            println!("Down={}", n);
            thread::sleep(Duration::from_secs(1));
            let scheduler_clone = scheduler.clone();
            scheduler.schedule(Task::new(
                move || countdown(n - 1, scheduler_clone.clone()),
                Some(Duration::from_secs(2)),
            ));
        }
    }

    fn countup(n: usize, scheduler: Arc<Scheduler>) {
        if n > 0 {
            println!("Up={}", n);
            thread::sleep(Duration::from_secs(2));
            let scheduler_clone = scheduler.clone();
            scheduler.schedule(Task::new(
                move || countup(n - 1, scheduler_clone.clone()),
                None,
            ))
        }
    }

    #[test]
    fn test() {
        let scheduler = Scheduler::new();

        {
            let scheduler_clone = scheduler.clone();
            scheduler.schedule(Task::new(move || countdown(5, scheduler_clone), None));
        }

        {
            let scheduler_clone = scheduler.clone();
            scheduler.schedule(Task::new(move || countup(3, scheduler_clone), None));
        }

        scheduler.run();
    }
}
