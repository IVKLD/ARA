pub use ara_compiler::{Component, component};
pub use ara_core as core;
pub use ara_render::{bootstrap, run};

pub mod prelude {
    pub use ara_compiler::{Component, component};
    pub use ara_core::{
        Computed, Inject, Signal,
        signal::{Get, GetUntracked, Set, Update, With},
    };
    pub use ara_render::bootstrap;
}

pub mod config {
    pub use ara_compiler::{Component, component};
    pub use ara_core::{
        Construct, DefineComponent, DefineService, Injector, Mount, NodeId, ProvideInRoot,
        SceneGraph,
    };
}
