---
name: rust-strict
description: Reglas obligatorias para escribir, compilar y probar código Rust en este proyecto.
---
- Usa `anyhow::Result` en la aplicación y `thiserror` para errores de dominio en módulos.
- Prohibido usar `.unwrap()` o `.expect()` en código de producción; propaga errores con `?`.
- Mantén separación estricta entre hilos/tareas: el loop BLE no debe bloquearse con escritura de archivos ni peticiones HTTP.
- Antes de dar por terminada cualquier tarea, ejecuta `cargo check --tests` y `cargo test`.