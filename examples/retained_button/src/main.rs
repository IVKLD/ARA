use ara_core::{
    SceneGraph, Signal,
    signal::{Get, Update},
};

fn main() {
    let scene = SceneGraph::new();
    let clicks: Signal<u32> = Signal::new(0);

    // 1. Фон (серый) — создаётся ровно один раз
    let _bg_id = scene.add_node([0.0, 0.0, 4000.0, 4000.0], [0.18, 0.18, 0.20, 1.0], None);

    // 2. Кнопка (синяя) — создаётся ровно один раз
    let clicks_for_click = clicks.clone();
    let btn_id = scene.add_node(
        [250.0, 220.0, 300.0, 120.0],
        [0.2, 0.0, 0.8, 1.0],
        Some(Box::new(move || {
            clicks_for_click.update(|c| *c += 1);
        })),
    );

    // 3. Хирургический реактивный эффект
    let scene_clone = scene.clone();
    reactive_graph::effect::Effect::new(move |_| {
        let count = clicks.get();
        // Напрямую меняем цвет ноды кнопки по клику без вызова каких-либо render()!
        let g = (count as f32 * 0.1).min(1.0);
        scene_clone.write().set_color(btn_id, [0.2, g, 0.8, 1.0]);
    });

    ara_render::run(scene);
}
