use ara::prelude::*;

pub struct CounterService {
    pub clicks: Signal<i32>,
}
impl CounterService {
    pub fn inc(&self) { self.clicks.update(|c| *c += 1); }
    pub fn dec(&self) { self.clicks.update(|c| *c -= 1); }
    pub fn reset(&self) { self.clicks.set(0); }
}
