use crate::analytics::SolveReport;

pub async fn analyze_solve_with_gemini(report: &SolveReport) -> anyhow::Result<String> {
    dotenvy::dotenv().ok();
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY not found in environment or .env"))?;

    let compact_report = serde_json::json!({
        "fases": report.phases,
        "tiempos": {
            "total_time_ms": report.total_time_ms,
            "active_time_ms": report.active_time_ms,
            "tps_raw": report.tps_raw,
            "tps_effective": report.tps_effective,
        },
        "pausas_criticas": report.pauses.iter().filter(|p| p.duration_ms > 350).collect::<Vec<_>>(),
        "anomalies": report.anomalies,
    });
    let report_json = serde_json::to_string(&compact_report)?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "systemInstruction": {
            "parts": [
                {
                    "text": "Eres un entrenador experto de speedcubing (método CFOP). Analiza el informe del solve proporcionado y responde de forma concisa identificando: \n- Mayor pérdida de tiempo del solve.\n- Par F2L con peor lookahead o eficiencia.\n- Diagnóstico de fluidez (TPS efectivo vs pausas).\n- Un ejercicio práctico concreto para mejorar ese fallo."
                }
            ]
        },
        "contents": [
            {
                "parts": [
                    {
                        "text": format!("Aquí está el reporte del solve en formato JSON compacto:\n{}", report_json)
                    }
                ]
            }
        ]
    });

    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    let text = resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse response text from Gemini API"))?
        .to_string();

    Ok(text)
}
