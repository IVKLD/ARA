use std::sync::Arc;
use ara_core::{SceneGraph, flush_effects};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use crate::pipeline::Renderer;

struct App {
    scene: SceneGraph,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    cursor_pos: (f32, f32),
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("ARA Retained UI (Wayland + Vulkan)")
                .with_inner_size(LogicalSize::new(800.0, 600.0));
            let window = Arc::new(
                event_loop
                    .create_window(attrs)
                    .expect("Failed to create Wayland window"),
            );
            let renderer = pollster::block_on(Renderer::new(window.clone()));
            window.request_redraw();
            self.window = Some(window);
            self.renderer = Some(renderer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let (x, y) = self.cursor_pos;
                let hit = self.scene.hit_test(x, y);
                if let Some(id) = hit {
                    {
                        let guard = self.scene.read();
                        if let Some(node) = guard.nodes.get(id.0)
                            && let Some(cb) = &node.on_click
                        {
                            cb();
                        }
                    }
                    flush_effects();
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                flush_effects();
                if let Some(renderer) = &mut self.renderer {
                    match renderer.render(&self.scene) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            let size = renderer.size;
                            renderer.resize(size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(_) => {}
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn run(scene: SceneGraph) {
    flush_effects();
    let event_loop = EventLoop::new().expect("Failed to create Wayland EventLoop");
    let mut app = App {
        scene,
        window: None,
        renderer: None,
        cursor_pos: (0.0, 0.0),
    };
    event_loop.run_app(&mut app).expect("EventLoop error");
}
