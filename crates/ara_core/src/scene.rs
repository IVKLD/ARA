use std::{
    cell::RefCell,
    sync::{Arc, Once},
};
use any_spawner::{CustomExecutor, Executor, PinnedFuture, PinnedLocalFuture};
use futures::{
    executor::{LocalPool, LocalSpawner, ThreadPool},
    task::{LocalSpawnExt, SpawnExt},
};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use slotmap::{DefaultKey, SlotMap};

struct SyncReactiveExecutor {
    thread_pool: ThreadPool,
}

thread_local! {
    static LOCAL_POOL: RefCell<LocalPool> = RefCell::new(LocalPool::new());
    static LOCAL_SPAWNER: LocalSpawner = LOCAL_POOL.with(|pool| pool.borrow().spawner());
}

impl CustomExecutor for SyncReactiveExecutor {
    fn spawn(&self, fut: PinnedFuture<()>) {
        let _ = self.thread_pool.spawn(fut);
        self.poll_local();
    }

    fn spawn_local(&self, fut: PinnedLocalFuture<()>) {
        LOCAL_SPAWNER.with(|spawner| {
            let _ = spawner.spawn_local(fut);
        });
        self.poll_local();
    }

    fn poll_local(&self) {
        LOCAL_POOL.with(|pool| {
            if let Ok(mut pool) = pool.try_borrow_mut() {
                pool.run_until_stalled();
            }
        });
    }
}

pub fn init_reactive_runtime() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if let Ok(thread_pool) = ThreadPool::new() {
            let _ = Executor::init_custom_executor(SyncReactiveExecutor { thread_pool });
        }
    });
}

pub fn flush_effects() {
    Executor::poll_local();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub DefaultKey);

pub struct SceneNode {
    pub bounds: [f32; 4],
    pub color: [f32; 4],
    pub text: Option<String>,
    pub text_color: [f32; 4],
    pub text_scale: f32,
    pub on_click: Option<Box<dyn Fn() + Send + Sync>>,
}

impl SceneNode {
    pub fn new(
        bounds: [f32; 4],
        color: [f32; 4],
        on_click: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> Self {
        Self {
            bounds,
            color,
            text: None,
            text_color: [1.0, 1.0, 1.0, 1.0],
            text_scale: 3.0,
            on_click,
        }
    }
}

#[derive(Default)]
pub struct SceneGraphInner {
    pub nodes: SlotMap<DefaultKey, SceneNode>,
    pub order: Vec<NodeId>,
}

impl SceneGraphInner {
    pub fn add_node(
        &mut self,
        bounds: [f32; 4],
        color: [f32; 4],
        on_click: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> NodeId {
        let key = self.nodes.insert(SceneNode::new(bounds, color, on_click));
        let id = NodeId(key);
        self.order.push(id);
        id
    }

    pub fn set_color(&mut self, id: NodeId, color: [f32; 4]) {
        if let Some(node) = self.nodes.get_mut(id.0) {
            node.color = color;
        }
    }

    pub fn set_text(&mut self, id: NodeId, text: impl Into<String>) {
        if let Some(node) = self.nodes.get_mut(id.0) {
            node.text = Some(text.into());
        }
    }

    pub fn set_text_style(&mut self, id: NodeId, color: [f32; 4], scale: f32) {
        if let Some(node) = self.nodes.get_mut(id.0) {
            node.text_color = color;
            node.text_scale = scale;
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<NodeId> {
        for &id in self.order.iter().rev() {
            if let Some(node) = self.nodes.get(id.0) {
                let [bx, by, bw, bh] = node.bounds;
                if x >= bx && x <= bx + bw && y >= by && y <= by + bh && node.on_click.is_some() {
                    return Some(id);
                }
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct SceneGraph {
    inner: Arc<RwLock<SceneGraphInner>>,
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneGraph {
    pub fn new() -> Self {
        init_reactive_runtime();
        Self {
            inner: Arc::new(RwLock::new(SceneGraphInner::default())),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, SceneGraphInner> {
        flush_effects();
        self.inner.read()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, SceneGraphInner> {
        self.inner.write()
    }

    pub fn add_node(
        &self,
        bounds: [f32; 4],
        color: [f32; 4],
        on_click: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> NodeId {
        self.inner.write().add_node(bounds, color, on_click)
    }

    pub fn set_color(&self, id: NodeId, color: [f32; 4]) {
        self.inner.write().set_color(id, color);
    }

    pub fn set_text(&self, id: NodeId, text: impl Into<String>) {
        self.inner.write().set_text(id, text);
    }

    pub fn set_text_style(&self, id: NodeId, color: [f32; 4], scale: f32) {
        self.inner.write().set_text_style(id, color, scale);
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<NodeId> {
        flush_effects();
        self.inner.read().hit_test(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::{Get, Signal, Update};

    #[test]
    fn retained_effect_updates_node_color_and_text_directly() {
        let scene = SceneGraph::new();
        let clicks: Signal<u32> = Signal::new(0);

        let clicks_cb = clicks.clone();
        let btn_id = scene.add_node(
            [100.0, 100.0, 200.0, 80.0],
            [0.2, 0.0, 0.8, 1.0],
            Some(Box::new(move || {
                clicks_cb.update(|c| *c += 1);
            })),
        );

        let scene_clone = scene.clone();
        reactive_graph::effect::Effect::new(move |_| {
            let count = clicks.get();
            let g = (count as f32 * 0.1).min(1.0);
            let mut w = scene_clone.write();
            w.set_color(btn_id, [0.2, g, 0.8, 1.0]);
            w.set_text(btn_id, format!("CLICKS: {count}"));
        });

        assert_eq!(scene.read().nodes[btn_id.0].color, [0.2, 0.0, 0.8, 1.0]);
        assert_eq!(
            scene.read().nodes[btn_id.0].text.as_deref(),
            Some("CLICKS: 0")
        );

        if let Some(id) = scene.hit_test(150.0, 120.0) {
            let guard = scene.read();
            if let Some(cb) = &guard.nodes[id.0].on_click {
                cb();
            }
        }

        assert_eq!(scene.read().nodes[btn_id.0].color, [0.2, 0.1, 0.8, 1.0]);
        assert_eq!(
            scene.read().nodes[btn_id.0].text.as_deref(),
            Some("CLICKS: 1")
        );
    }
}
