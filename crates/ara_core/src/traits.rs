use std::sync::Arc;
use parking_lot::RwLock;
use crate::{
    di::Construct,
    scene::{NodeId, SceneGraph},
};

pub struct ProvideInRoot;

pub trait DefineService: Construct + Send + Sync + 'static {
    type Scope;
}

pub trait Mount {
    fn mount(&self, scene: &Arc<RwLock<SceneGraph>>, parent_id: Option<NodeId>);
}

pub trait DefineComponent: Construct + Mount {}
