use std::{collections::VecDeque, time::{Duration, Instant}};

pub struct Tickrate {
    frame_times: VecDeque<Instant>
}

impl Tickrate {
    pub fn new() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(128)
        }
    }

    pub fn tick(&mut self) -> usize {
        let now = Instant::now();
        let last = now - Duration::from_secs(1);

        while self.frame_times.front().map_or(false, |t| *t < last) {
            self.frame_times.pop_front();
        }

        self.frame_times.push_back(now);
        self.frame_times.len()
    }
}