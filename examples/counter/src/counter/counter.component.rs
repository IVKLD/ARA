use ara::prelude::*;
use super::CounterService;

pub struct CounterComponent {
    pub srv: Inject<CounterService>,
}
