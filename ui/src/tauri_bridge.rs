use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::models::ScrambleStateDto;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], catch)]
    async fn listen(event: &str, handler: &js_sys::Function) -> Result<JsValue, JsValue>;
}

fn format_js_err(err: JsValue) -> String {
    err.as_string()
        .unwrap_or_else(|| format!("{err:?}"))
}

pub async fn invoke_no_args(cmd: &str) -> Result<JsValue, String> {
    invoke(cmd, JsValue::NULL)
        .await
        .map_err(format_js_err)
}

pub async fn invoke_with_args<T: Serialize>(cmd: &str, args: &T) -> Result<JsValue, String> {
    let js_args = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    invoke(cmd, js_args).await.map_err(format_js_err)
}

pub async fn invoke_get_current_scramble() -> Result<ScrambleStateDto, String> {
    let val = invoke_no_args("get_current_scramble").await?;
    serde_wasm_bindgen::from_value(val).map_err(|e| e.to_string())
}

pub async fn invoke_next_scramble() -> Result<String, String> {
    let val = invoke_no_args("next_scramble").await?;
    val.as_string()
        .ok_or_else(|| "Respuesta inválida al generar scramble".to_string())
}

pub async fn invoke_prev_scramble() -> Result<Option<String>, String> {
    let val = invoke_no_args("prev_scramble").await?;
    if val.is_null() || val.is_undefined() {
        return Ok(None);
    }
    Ok(val.as_string())
}

pub async fn invoke_start_ble_scan() -> Result<(), String> {
    invoke_no_args("start_ble_scan").await.map(|_| ())
}

pub async fn invoke_run_demo_solve() -> Result<(), String> {
    invoke_no_args("run_demo_solve").await.map(|_| ())
}

pub async fn invoke_request_ai_coach(report_json: &serde_json::Value) -> Result<String, String> {
    let payload = serde_json::json!({ "report": report_json });
    let val = invoke_with_args("request_ai_coach", &payload).await?;
    val.as_string()
        .ok_or_else(|| "Respuesta inválida del Coach IA".to_string())
}

pub async fn invoke_request_ai_session_coach(
    solves: &[serde_json::Value],
) -> Result<String, String> {
    let payload = serde_json::json!({ "solves": solves });
    let val = invoke_with_args("request_ai_session_coach", &payload).await?;
    val.as_string()
        .ok_or_else(|| "Respuesta inválida del Coach IA de sesión".to_string())
}

pub async fn invoke_record_manual_solve(total_time_ms: u64) -> Result<(), String> {
    let payload = serde_json::json!({ "totalTimeMs": total_time_ms });
    invoke_with_args("record_manual_solve", &payload)
        .await
        .map(|_| ())
}

pub async fn invoke_delete_solve(index: usize) -> Result<(), String> {
    let payload = serde_json::json!({ "index": index });
    invoke_with_args("delete_solve", &payload)
        .await
        .map(|_| ())
}

pub async fn invoke_create_session(name: Option<String>) -> Result<String, String> {
    let payload = serde_json::json!({ "name": name });
    let val = invoke_with_args("create_session", &payload).await?;
    val.as_string()
        .ok_or_else(|| "Respuesta inválida al crear serie".to_string())
}

pub async fn invoke_switch_session(name: String) -> Result<Vec<serde_json::Value>, String> {
    let payload = serde_json::json!({ "name": name });
    let val = invoke_with_args("switch_session", &payload).await?;
    serde_wasm_bindgen::from_value(val).map_err(|e| e.to_string())
}

pub async fn invoke_clear_active_session() -> Result<(), String> {
    invoke_no_args("clear_active_session").await.map(|_| ())
}

pub async fn invoke_export_session(solves: &[serde_json::Value]) -> Result<String, String> {
    let payload = serde_json::json!({ "solves": solves });
    let val = invoke_with_args("export_session", &payload).await?;
    val.as_string()
        .ok_or_else(|| "Respuesta inválida al exportar Excel".to_string())
}

pub async fn listen_event<T, F>(event_name: &str, mut on_payload: F) -> Result<(), String>
where
    T: DeserializeOwned + 'static,
    F: FnMut(T) + 'static,
{
    let closure = Closure::wrap(Box::new(move |event_obj: JsValue| {
        if let Ok(payload_val) = js_sys::Reflect::get(&event_obj, &JsValue::from_str("payload")) {
            if let Ok(parsed) = serde_wasm_bindgen::from_value::<T>(payload_val) {
                on_payload(parsed);
            }
        }
    }) as Box<dyn FnMut(JsValue)>);

    let func = closure
        .as_ref()
        .dyn_ref::<js_sys::Function>()
        .ok_or_else(|| "No se pudo convertir el closure a js_sys::Function".to_string())?;

    let result = listen(event_name, func).await.map_err(format_js_err);
    closure.forget();
    result.map(|_| ())
}
