use std::collections::VecDeque;

#[derive(Default)]
pub struct LoopDetector {
    last_actions: VecDeque<String>,
    max_actions: usize,
}

impl LoopDetector {
    pub fn new() -> Self {
        Self {
            last_actions: VecDeque::new(),
            max_actions: 20,
        }
    }

    pub fn record_action(&mut self, action: String) {
        self.last_actions.push_back(action);
        while self.last_actions.len() > self.max_actions {
            let _ = self.last_actions.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.last_actions.clear();
    }

    pub fn has_loop(&self) -> bool {
        if self.last_actions.len() < 6 {
            return false;
        }
        let tail: Vec<_> = self.last_actions.iter().rev().take(6).collect();
        tail[0] == tail[2] && tail[2] == tail[4] && tail[1] == tail[3] && tail[3] == tail[5]
    }
}
