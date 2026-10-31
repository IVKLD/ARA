use ara_core::{
    SceneGraph, Signal,
    signal::{Get, Set, Update},
};

fn main() {
    let scene = SceneGraph::new();
    let clicks: Signal<i32> = Signal::new(0);

    // 1. Фон окна (создаётся 1 раз)
    let _bg_id = scene.add_node([0.0, 0.0, 4000.0, 4000.0], [0.09, 0.10, 0.13, 1.0], None);

    // 2. Карточка счётчика (создаётся 1 раз)
    let _card_id = scene.add_node([160.0, 90.0, 480.0, 400.0], [0.14, 0.16, 0.21, 1.0], None);

    // 3. Табло с текстом количества кликов (создаётся 1 раз)
    let display_id = scene.add_node([200.0, 130.0, 400.0, 120.0], [0.07, 0.08, 0.11, 1.0], None);
    scene.set_text_style(display_id, [0.40, 0.92, 0.65, 1.0], 5.0);

    // 4. Кнопка "-1" (создаётся 1 раз)
    let clicks_dec = clicks.clone();
    let dec_btn_id = scene.add_node(
        [200.0, 290.0, 120.0, 80.0],
        [0.75, 0.22, 0.28, 1.0],
        Some(Box::new(move || {
            clicks_dec.update(|c| *c -= 1);
        })),
    );
    scene.set_text(dec_btn_id, "-1");
    scene.set_text_style(dec_btn_id, [1.0, 1.0, 1.0, 1.0], 4.0);

    // 5. Кнопка "RESET" (создаётся 1 раз)
    let clicks_reset = clicks.clone();
    let reset_btn_id = scene.add_node(
        [340.0, 290.0, 120.0, 80.0],
        [0.26, 0.29, 0.37, 1.0],
        Some(Box::new(move || {
            clicks_reset.set(0);
        })),
    );
    scene.set_text(reset_btn_id, "RESET");
    scene.set_text_style(reset_btn_id, [0.9, 0.9, 0.95, 1.0], 3.0);

    // 6. Кнопка "+1" (создаётся 1 раз)
    let clicks_inc = clicks.clone();
    let inc_btn_id = scene.add_node(
        [480.0, 290.0, 120.0, 80.0],
        [0.20, 0.48, 0.88, 1.0],
        Some(Box::new(move || {
            clicks_inc.update(|c| *c += 1);
        })),
    );
    scene.set_text(inc_btn_id, "+1");
    scene.set_text_style(inc_btn_id, [1.0, 1.0, 1.0, 1.0], 4.0);

    // 7. Хирургический реактивный эффект: обновляет текст табло и оттенок напрямую в SceneGraph
    let scene_clone = scene.clone();
    reactive_graph::effect::Effect::new(move |_| {
        let count = clicks.get();
        println!("Кликов: {count}");

        let mut graph = scene_clone.write();
        graph.set_text(display_id, format!("CLICKS: {count}"));

        // Меняем цвет текста табло в зависимости от знака счётчика
        let text_color = if count > 0 {
            [0.35, 0.95, 0.60, 1.0]
        } else if count < 0 {
            [0.98, 0.45, 0.45, 1.0]
        } else {
            [0.85, 0.88, 0.95, 1.0]
        };
        graph.set_text_style(display_id, text_color, 5.0);
    });

    ara_render::run(scene);
}
