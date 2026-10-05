# Tauri with a Svelte + TypeScript frontend, not Rust/WASM

Hikestat is a local, single-user desktop app built with Tauri. The backend is Rust (it depends on the `hikestat` library from `hikestats-ingester`), but the webview UI is written in Svelte + TypeScript rather than a Rust/WASM framework (Leptos, Dioxus, Yew). The app's future is maps (MapLibre GL, track rendering, chart ↔ map interaction) and charts, and those libraries are JavaScript: driving them through WASM bindings would mean hand-written glue for the most interactive part of the app. Svelte was chosen over React for its lightness.

## Consequences

- Domain logic (aggregation by period in the start time zone, ISO weeks) stays in Rust, in the `hikestat-app` crate, so it is unit-tested in Rust; the frontend only displays.
- Types crossing the Tauri command boundary should be generated from Rust (e.g. `tauri-specta`) rather than written twice.
