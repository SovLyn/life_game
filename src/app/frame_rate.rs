use std::collections::VecDeque;

pub struct FrameRate {
    deque: VecDeque<f64>,
    max_store: usize,
}

impl FrameRate {
    pub fn new(max_store: usize) -> Self {
        if max_store < 2 {
            panic!("max_store must be greater than 1");
        }
        Self {
            deque: VecDeque::new(),
            max_store,
        }
    }

    pub fn add(&mut self, frame_time: f64) {
        self.deque.push_back(frame_time);
        if self.deque.len() > self.max_store {
            self.deque.pop_front();
        }
    }

    pub fn get_fps(&self) -> f64 {
        if self.deque.len() <= 1 {
            0.0
        } else {
            let n = self.deque.len();
            let total_time = self.deque[n - 1] - self.deque[0];
            (self.deque.len() - 1) as f64 / total_time
        }
    }
}
