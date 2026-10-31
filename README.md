# ARA (`ara-rs`) — Retained UI Engine

Экспериментальное графическое ядро пользовательского интерфейса на Rust по схеме **Retained UI (No-React)** со строгим стеком **Pure Wayland + Vulkan**.

## Архитектурные принципы

- **Никаких виртуальных деревьев и `render()`-функций**: UI-ноды создаются один раз и постоянно живут в памяти внутри [`SceneGraph`](crates/ara_core/src/scene.rs) на базе `slotmap::SlotMap`.
- **Хирургическая реактивность**: реактивные сигналы (`reactive_graph`) мутируют свойства конкретных нод (`color`, `text`, `bounds`) напрямую в памяти по 8-байтному ключу `NodeId` через одноразовый `Effect`.
- **Pure Wayland + Vulkan**: оконный бэкенд `winit` собран строго с `wayland` (без X11/XWayland), рендеринг выполняется через `wgpu` на бэкенде `Vulkan`.

## Структура Workspace

- `crates/ara_core` — `SceneGraph`, `SceneNode`, `NodeId`, синхронный реактивный рантайм и реэкспорт `Signal<T>` / `Computed<T>`.
- `crates/ara_render` — `winit` Wayland цикл событий, `hit_test` диспетчеризация кликов и `wgpu` Vulkan-пайплайн отрисовки прямоугольников и текста.
- `examples/retained_button` — минимальный пример прямого изменения цвета кнопки в памяти через `Effect`.
- `examples/counter` — интерактивный счётчик (`-1`, `RESET`, `+1`) с хирургическим обновлением текста и цвета табло.

## Запуск (NixOS / Nix Flakes)

```bash
# Запуск примера со счётчиком
nix develop -c cargo run -p counter

# Запуск базового примера с кнопкой
nix develop -c cargo run -p retained_button

# Запуск тестов
nix develop -c cargo test
```
