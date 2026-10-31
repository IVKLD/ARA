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
use taffy::prelude::*;

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
    pub taffy_node: Option<taffy::NodeId>,
    pub parent: Option<NodeId>,
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
            taffy_node: None,
            parent: None,
        }
    }
}

pub struct SceneGraphInner {
    pub nodes: SlotMap<DefaultKey, SceneNode>,
    pub order: Vec<NodeId>,
    pub taffy: TaffyTree<()>,
    pub root_taffy: Option<taffy::NodeId>,
    pub viewport_size: (f32, f32),
}

impl Default for SceneGraphInner {
    fn default() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            order: Vec::new(),
            taffy: TaffyTree::new(),
            root_taffy: None,
            viewport_size: (800.0, 600.0),
        }
    }
}

impl SceneGraphInner {
    fn ensure_root_taffy(&mut self) -> taffy::NodeId {
        if let Some(root) = self.root_taffy {
            root
        } else {
            let (w, h) = self.viewport_size;
            let root = self
                .taffy
                .new_leaf(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    justify_content: Some(JustifyContent::Center),
                    align_items: Some(AlignItems::Center),
                    size: Size {
                        width: Dimension::Length(w),
                        height: Dimension::Length(h),
                    },
                    ..Default::default()
                })
                .expect("Failed to create root taffy node");
            self.root_taffy = Some(root);
            root
        }
    }

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

    pub fn add_styled_node(
        &mut self,
        parent: Option<NodeId>,
        style: Style,
        color: [f32; 4],
        text_color: [f32; 4],
        text_scale: f32,
        on_click: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> NodeId {
        let taffy_node = self
            .taffy
            .new_leaf(style)
            .expect("Failed to create taffy node");

        let parent_taffy = parent
            .and_then(|pid| self.nodes.get(pid.0))
            .and_then(|pnode| pnode.taffy_node)
            .unwrap_or_else(|| self.ensure_root_taffy());

        let _ = self.taffy.add_child(parent_taffy, taffy_node);

        let mut scene_node = SceneNode::new([0.0, 0.0, 0.0, 0.0], color, on_click);
        scene_node.text_color = text_color;
        scene_node.text_scale = text_scale;
        scene_node.taffy_node = Some(taffy_node);
        scene_node.parent = parent;

        let key = self.nodes.insert(scene_node);
        let id = NodeId(key);
        self.order.push(id);
        self.recompute_layout();
        id
    }

    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_size = (width, height);
        if let Some(root) = self.root_taffy
            && let Ok(mut style) = self.taffy.style(root).cloned()
        {
            style.size = Size {
                width: Dimension::Length(width),
                height: Dimension::Length(height),
            };
            let _ = self.taffy.set_style(root, style);
        }
        self.recompute_layout();
    }

    pub fn recompute_layout(&mut self) {
        let Some(root) = self.root_taffy else {
            return;
        };
        let (w, h) = self.viewport_size;
        let available = Size {
            width: AvailableSpace::Definite(w),
            height: AvailableSpace::Definite(h),
        };
        if self.taffy.compute_layout(root, available).is_ok() {
            let order = self.order.clone();
            for id in order {
                let (t_node, parent_id) = match self.nodes.get(id.0) {
                    Some(n) => (n.taffy_node, n.parent),
                    None => continue,
                };
                if let Some(tn) = t_node
                    && let Ok(layout) = self.taffy.layout(tn)
                {
                    let (px, py) = parent_id
                        .and_then(|pid| self.nodes.get(pid.0))
                        .map(|p| (p.bounds[0], p.bounds[1]))
                        .unwrap_or((0.0, 0.0));
                    if let Some(node) = self.nodes.get_mut(id.0) {
                        node.bounds = [
                            px + layout.location.x,
                            py + layout.location.y,
                            layout.size.width,
                            layout.size.height,
                        ];
                    }
                }
            }
        }
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

    pub fn update_text(&mut self, id: NodeId, text: impl Into<String>) {
        self.set_text(id, text);
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

    pub fn into_shared(self) -> Arc<RwLock<SceneGraph>> {
        Arc::new(RwLock::new(self))
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

    pub fn add_styled_node(
        &self,
        parent: Option<NodeId>,
        style: Style,
        color: [f32; 4],
        text_color: [f32; 4],
        text_scale: f32,
        on_click: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> NodeId {
        self.inner
            .write()
            .add_styled_node(parent, style, color, text_color, text_scale, on_click)
    }

    pub fn set_viewport_size(&self, width: f32, height: f32) {
        self.inner.write().set_viewport_size(width, height);
    }

    pub fn set_color(&self, id: NodeId, color: [f32; 4]) {
        self.inner.write().set_color(id, color);
    }

    pub fn set_text(&self, id: NodeId, text: impl Into<String>) {
        self.inner.write().set_text(id, text);
    }

    pub fn update_text(&self, id: NodeId, text: impl Into<String>) {
        self.inner.write().update_text(id, text);
    }

    pub fn set_text_style(&self, id: NodeId, color: [f32; 4], scale: f32) {
        self.inner.write().set_text_style(id, color, scale);
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<NodeId> {
        flush_effects();
        self.inner.read().hit_test(x, y)
    }
}
