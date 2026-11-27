use std::collections::BTreeMap;

use crate::analytics::{phase_str, SolveReport};

async fn call_gemini_api(system_prompt: &str, user_text: &str) -> anyhow::Result<String> {
    dotenvy::dotenv().ok();
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY not found in environment or .env"))?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "systemInstruction": {
            "parts": [
                {
                    "text": system_prompt
                }
            ]
        },
        "contents": [
            {
                "parts": [
                    {
                        "text": user_text
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

pub async fn analyze_solve_with_gemini(report: &SolveReport) -> anyhow::Result<String> {
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

    let system_prompt = "Eres un entrenador experto de speedcubing (método CFOP). Analiza el informe del solve proporcionado y responde de forma concisa identificando: \n- Mayor pérdida de tiempo del solve.\n- Par F2L con peor lookahead o eficiencia.\n- Diagnóstico de fluidez (TPS efectivo vs pausas).\n- Un ejercicio práctico concreto para mejorar ese fallo.";
    let user_text = format!(
        "Aquí está el reporte del solve en formato JSON compacto:\n{}",
        report_json
    );

    call_gemini_api(system_prompt, &user_text).await
}

fn trimmed_average_ms(solves: &[SolveReport], window: usize) -> Option<u64> {
    if solves.len() < window || window < 3 {
        return None;
    }
    let start = solves.len() - window;
    let mut times: Vec<u64> = solves[start..].iter().map(|s| s.total_time_ms).collect();
    times.sort_unstable();
    let trimmed = &times[1..times.len() - 1];
    let sum: u64 = trimmed.iter().sum();
    Some(sum / trimmed.len() as u64)
}

pub async fn analyze_session_with_gemini(solves: &[SolveReport]) -> anyhow::Result<String> {
    if solves.is_empty() {
        return Err(anyhow::anyhow!("No hay solves en la sesión para analizar"));
    }

    let total_solves = solves.len();
    let pb_ms = solves.iter().map(|s| s.total_time_ms).min();
    let ao5_ms = trimmed_average_ms(solves, 5);
    let ao12_ms = trimmed_average_ms(solves, 12);

    let mut phase_acc: BTreeMap<&'static str, (u64, f32, usize, usize)> = BTreeMap::new();
    let mut total_critical_pauses = 0usize;
    let mut total_lockups = 0usize;
    let mut total_y_rotations_f2l = 0usize;

    for report in solves {
        total_critical_pauses += report.pauses.iter().filter(|p| p.duration_ms > 350).count();
        total_lockups += report.lockups.len();
        total_y_rotations_f2l += report.rotation_tax.total_y_rotations_f2l;

        for p in &report.phases {
            let name = phase_str(p.phase);
            let entry = phase_acc.entry(name).or_insert((0, 0.0, 0, 0));
            entry.0 += p.duration_ms;
            entry.1 += p.tps_effective;
            entry.2 += p.move_count;
            entry.3 += 1;
        }
    }

    let phase_averages: serde_json::Map<String, serde_json::Value> = phase_acc
        .into_iter()
        .map(|(name, (dur_sum, tps_sum, moves_sum, count))| {
            let c = count.max(1) as f32;
            (
                name.to_string(),
                serde_json::json!({
                    "avg_duration_ms": dur_sum / count.max(1) as u64,
                    "avg_tps_effective": (tps_sum / c * 100.0).round() / 100.0,
                    "avg_moves": (moves_sum as f32 / c * 10.0).round() / 10.0,
                }),
            )
        })
        .collect();

    let recent_start = solves.len().saturating_sub(10);
    let recent_solves: Vec<serde_json::Value> = solves[recent_start..]
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            serde_json::json!({
                "index": recent_start + idx + 1,
                "total_time_ms": s.total_time_ms,
                "tps_effective": s.tps_effective,
                "pauses_gt_350ms": s.pauses.iter().filter(|p| p.duration_ms > 350).count(),
                "lockups": s.lockups.len(),
                "f2l_y_rotations": s.rotation_tax.total_y_rotations_f2l,
                "anomalies": s.anomalies,
            })
        })
        .collect();

    let session_summary = serde_json::json!({
        "total_solves": total_solves,
        "pb_ms": pb_ms,
        "ao5_ms": ao5_ms,
        "ao12_ms": ao12_ms,
        "phase_averages": phase_averages,
        "accumulated_issues": {
            "total_critical_pauses_gt_350ms": total_critical_pauses,
            "total_lockups": total_lockups,
            "total_f2l_y_rotations": total_y_rotations_f2l,
        },
        "recent_solves": recent_solves,
    });

    let system_prompt = "Eres un entrenador experto de speedcubing (método CFOP). Analiza el resumen completo de esta sesión de práctica y responde en Markdown claro con: \n- **Patrones recurrentes de la sesión** (dónde se pierde tiempo de forma consistente).\n- **Fase CFOP más débil** según los promedios de duración y TPS.\n- **Diagnóstico de fluidez global** (pausas críticas, lockups y rotaciones en F2L).\n- **Plan de práctica concreto** (2 o 3 drills específicos para la próxima sesión).";
    let user_text = format!(
        "Resumen de sesión en formato JSON:\n{}",
        serde_json::to_string(&session_summary)?
    );

    call_gemini_api(system_prompt, &user_text).await
}
