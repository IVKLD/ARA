#[path = "counter.service.rs"]
mod counter_service;
#[path = "counter.component.rs"]
mod counter_component;

pub use counter_component::CounterComponent;
pub use counter_service::CounterService;

use ara::{config::*, prelude::*};

impl DefineService for CounterService {
    type Scope = ProvideInRoot;
}

impl Construct for CounterService {
    fn construct(_inj: &Injector) -> Self {
        Self {
            clicks: Signal::new(0),
        }
    }
}

ara::component! {
    component: CounterComponent {
        srv: Inject<CounterService>,
    },
    template: "src/counter/counter.component.html",
    style: "src/counter/counter.component.scss",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angular_v22_counter_mounts_and_reacts_to_clicks() {
        let injector = Injector::new();
        let scene_graph = SceneGraph::new();
        let shared = scene_graph.clone().into_shared();

        let comp = CounterComponent::construct(&injector);
        comp.mount(&shared, None);

        // Initial render from template: "CLICKS: 0"
        let has_zero = scene_graph
            .read()
            .nodes
            .values()
            .any(|n| n.text.as_deref() == Some("CLICKS: 0"));
        assert!(has_zero, "Expected display node with 'CLICKS: 0'");

        // Service method updates signal and surgical Effect updates retained node text
        comp.srv.inc();
        comp.srv.inc();
        let has_two = scene_graph
            .read()
            .nodes
            .values()
            .any(|n| n.text.as_deref() == Some("CLICKS: 2"));
        assert!(has_two, "Expected display node to update to 'CLICKS: 2'");
    }
}
