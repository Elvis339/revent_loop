use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

pub struct Task {
    callback: Box<dyn Fn() -> Option<Task>>,
}

impl Task {
    fn new(callback: impl Fn() -> Option<Task> + 'static) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }

    fn run(&self) -> Option<Task> {
        (self.callback)()
    }
}

#[derive(Default)]
pub struct Scheduler {
    ready_fns: VecDeque<Task>,
}

impl Scheduler {
    fn enqueue(&mut self, task: Task) {
        self.ready_fns.push_back(task);
    }

    fn run_next(&mut self) {
        if let Some(mut task) = self.ready_fns.pop_front() {
            if let Some(next_task) = task.run() {
                self.ready_fns.push_back(next_task);
            }
        }
    }

    fn run(&mut self) {
        while !self.ready_fns.is_empty() {
            self.run_next();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn countdown(n: usize, s: Rc<RefCell<Scheduler>>) -> Option<Task> {
        if n > 0 {
            println!("Countdown={}", n);
            thread::sleep(Duration::from_secs(1));
            let cloned = s.clone();
            Some(Task::new(move || countdown(n - 1, cloned.clone())))
        } else {
            None
        }
    }

    fn countup(stop: usize, x: usize, s: Rc<RefCell<Scheduler>>) -> Option<Task> {
        if x < stop {
            println!("Up={}", x);
            thread::sleep(Duration::from_secs(1));
            let cloned = s.clone();
            Some(Task::new(move || countup(stop, x + 1, cloned.clone())))
        } else {
            None
        }
    }

    #[test]
    fn test() {
        let scheduler = Rc::new(RefCell::new(Scheduler::default()));
        {
            let sc_ref = Rc::clone(&scheduler);
            scheduler
                .borrow_mut()
                .enqueue(Task::new(move || countdown(3, sc_ref.clone())));
        }

        {
            let mut x = 0;
            let sc_ref = Rc::clone(&scheduler);
            scheduler
                .borrow_mut()
                .enqueue(Task::new(move || countup(3, x, sc_ref.clone())))
        }

        while !scheduler.borrow().ready_fns.is_empty() {
            scheduler.borrow_mut().run_next();
        }
    }
}
