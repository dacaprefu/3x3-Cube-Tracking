use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
