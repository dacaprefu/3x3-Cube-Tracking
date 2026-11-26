use crate::analytics::models::SolveReport;
use crate::cfop::phase::CfopPhase;
use rust_xlsxwriter::Workbook;

pub fn export_session_to_excel(path: &str, solves: &[SolveReport]) -> anyhow::Result<()> {
    let mut workbook = Workbook::new();

    // 1. Hoja Solves_Detallados
    let sheet1 = workbook.add_worksheet();
    sheet1.set_name("Solves_Detallados")?;

    let headers = [
        "ID",
        "Fecha",
        "Scramble",
        "Tiempo Total (ms)",
        "TPS Bruto",
        "TPS Efectivo",
        "Rotaciones F2L",
        "Cruz - ms",
        "Cruz - giros",
        "F2L 1 - ms",
        "F2L 1 - giros",
        "F2L 2 - ms",
        "F2L 2 - giros",
        "F2L 3 - ms",
        "F2L 3 - giros",
        "F2L 4 - ms",
        "F2L 4 - giros",
        "X-Cross",
        "OLL - Reconocimiento (ms)",
        "OLL - Ejecución (ms)",
        "OLL - Skip",
        "PLL - Reconocimiento (ms)",
        "PLL - Ejecución (ms)",
        "PLL - Skip",
        "Pausas >350ms",
        "Etiquetas de Anomalías",
    ];

    for (col_idx, header) in headers.iter().enumerate() {
        sheet1.write_string(0, col_idx as u16, *header)?;
    }

    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    for (row_idx, solve) in solves.iter().enumerate() {
        let r = (row_idx + 1) as u32;

        // ID
        sheet1.write_number(r, 0, (row_idx + 1) as f64)?;
        // Fecha
        sheet1.write_string(r, 1, &now_str)?;
        // Scramble
        sheet1.write_string(r, 2, "N/A")?;
        // Tiempo Total (ms)
        sheet1.write_number(r, 3, solve.total_time_ms as f64)?;
        // TPS Bruto
        sheet1.write_number(r, 4, solve.tps_raw as f64)?;
        // TPS Efectivo
        sheet1.write_number(r, 5, solve.tps_effective as f64)?;
        // Rotaciones F2L
        sheet1.write_number(r, 6, solve.rotation_tax.total_y_rotations_f2l as f64)?;

        let get_phase_metrics = |phase: CfopPhase| -> Option<&crate::analytics::models::PhaseMetrics> {
            solve.phases.iter().find(|p| p.phase == phase)
        };

        // Cruz
        let cross = get_phase_metrics(CfopPhase::Cross);
        sheet1.write_number(r, 7, cross.map_or(0.0, |p| p.duration_ms as f64))?;
        sheet1.write_number(r, 8, cross.map_or(0.0, |p| p.move_count as f64))?;

        // F2L 1-4
        let f2l_1 = get_phase_metrics(CfopPhase::F2L_1);
        let f2l_2 = get_phase_metrics(CfopPhase::F2L_2);
        let f2l_3 = get_phase_metrics(CfopPhase::F2L_3);
        let f2l_4 = get_phase_metrics(CfopPhase::F2L_4);

        sheet1.write_number(r, 9, f2l_1.map_or(0.0, |p| p.duration_ms as f64))?;
        sheet1.write_number(r, 10, f2l_1.map_or(0.0, |p| p.move_count as f64))?;

        sheet1.write_number(r, 11, f2l_2.map_or(0.0, |p| p.duration_ms as f64))?;
        sheet1.write_number(r, 12, f2l_2.map_or(0.0, |p| p.move_count as f64))?;

        sheet1.write_number(r, 13, f2l_3.map_or(0.0, |p| p.duration_ms as f64))?;
        sheet1.write_number(r, 14, f2l_3.map_or(0.0, |p| p.move_count as f64))?;

        sheet1.write_number(r, 15, f2l_4.map_or(0.0, |p| p.duration_ms as f64))?;
        sheet1.write_number(r, 16, f2l_4.map_or(0.0, |p| p.move_count as f64))?;

        // X-Cross flag (si F2L_1 no tiene duración o giros y se resolvió en cruz)
        let is_x_cross = f2l_1.map_or(false, |p| p.duration_ms == 0 && p.move_count == 0);
        sheet1.write_string(r, 17, if is_x_cross { "Sí" } else { "No" })?;

        // OLL
        let oll = get_phase_metrics(CfopPhase::OLL);
        sheet1.write_number(r, 18, oll.and_then(|p| p.recognition_time_ms).map_or(0.0, |v| v as f64))?;
        sheet1.write_number(r, 19, oll.and_then(|p| p.execution_time_ms).map_or(0.0, |v| v as f64))?;
        let oll_skip = oll.map_or(true, |p| p.duration_ms == 0);
        sheet1.write_string(r, 20, if oll_skip { "Sí" } else { "No" })?;

        // PLL
        let pll = get_phase_metrics(CfopPhase::PLL);
        sheet1.write_number(r, 21, pll.and_then(|p| p.recognition_time_ms).map_or(0.0, |v| v as f64))?;
        sheet1.write_number(r, 22, pll.and_then(|p| p.execution_time_ms).map_or(0.0, |v| v as f64))?;
        let pll_skip = pll.map_or(true, |p| p.duration_ms == 0);
        sheet1.write_string(r, 23, if pll_skip { "Sí" } else { "No" })?;

        // Pausas >350ms
        let pauses_count = solve.pauses.iter().filter(|p| p.duration_ms > 350).count();
        sheet1.write_number(r, 24, pauses_count as f64)?;

        // Etiquetas de Anomalías
        let anomalies_str = solve.anomalies.join("; ");
        sheet1.write_string(r, 25, &anomalies_str)?;
    }

    // 2. Hoja Resumen_Sesion
    let sheet2 = workbook.add_worksheet();
    sheet2.set_name("Resumen_Sesion")?;

    sheet2.write_string(0, 0, "Métrica")?;
    sheet2.write_string(0, 1, "Valor")?;

    let num_solves = solves.len();
    let end_row = std::cmp::max(2, num_solves + 1);

    let f1 = format!("=COUNTA(Solves_Detallados!A2:A{})", end_row);
    let f2 = format!("=AVERAGE(Solves_Detallados!D2:D{})", end_row);
    let f3 = format!("=AVERAGE(Solves_Detallados!E2:E{})", end_row);
    let f4 = format!("=AVERAGE(Solves_Detallados!F2:F{})", end_row);
    let f5 = format!("=AVERAGE(Solves_Detallados!G2:G{})", end_row);
    let f6 = format!(r#"=COUNTIF(Solves_Detallados!Z2:Z{}, "*")"#, end_row);

    sheet2.write_string(1, 0, "Total Solves")?;
    sheet2.write_formula(1, 1, f1.as_str())?;

    sheet2.write_string(2, 0, "Tiempo Total Promedio (ms)")?;
    sheet2.write_formula(2, 1, f2.as_str())?;

    sheet2.write_string(3, 0, "TPS Bruto Promedio")?;
    sheet2.write_formula(3, 1, f3.as_str())?;

    sheet2.write_string(4, 0, "TPS Efectivo Promedio")?;
    sheet2.write_formula(4, 1, f4.as_str())?;

    sheet2.write_string(5, 0, "Rotaciones F2L Promedio")?;
    sheet2.write_formula(5, 1, f5.as_str())?;

    sheet2.write_string(6, 0, "Conteo de Anomalías")?;
    sheet2.write_formula(6, 1, f6.as_str())?;

    workbook.save(path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::analyzer::SolveAnalyzer;
    use crate::cfop::tracker::CfopTracker;
    use crate::cube::moves::{parse_moves, Move};

    fn generate_simulated_solve() -> SolveReport {
        let mut tracker = CfopTracker::new();

        let t_perm = parse_moves("R U R' U' R' F R2 U' R' U' R U R' F'").unwrap();
        let sune = parse_moves("R U R' U R U2 R'").unwrap();
        let sune_inv = parse_moves("R U2 R' U' R U' R'").unwrap();
        let displace = parse_moves("R U R' U'").unwrap();
        let insert = parse_moves("y' U R U' R'").unwrap();
        let cross_scramble = parse_moves("D'").unwrap();
        let cross_solve = parse_moves("D").unwrap();

        let mut scramble_sequence = Vec::new();
        scramble_sequence.extend_from_slice(&t_perm);
        scramble_sequence.extend_from_slice(&sune_inv);
        for _ in 0..4 {
            scramble_sequence.extend_from_slice(&displace);
            scramble_sequence.push(Move::Y);
        }
        scramble_sequence.extend_from_slice(&cross_scramble);

        tracker.apply_scramble(&scramble_sequence);
        tracker.start_inspection(1000);
        tracker.start_solve(2000);

        let mut current_time = 2000;

        for m in cross_solve {
            current_time += 300;
            tracker.on_move(m, current_time);
        }

        for _ in 0..4 {
            for &m in &insert {
                current_time += 150;
                tracker.on_move(m, current_time);
            }
        }

        for &m in &sune {
            current_time += 150;
            tracker.on_move(m, current_time);
        }

        for &m in &t_perm {
            current_time += 150;
            tracker.on_move(m, current_time);
        }

        let analyzer = SolveAnalyzer::new();
        analyzer.analyze(&tracker).unwrap()
    }

    #[test]
    fn test_export_session_to_excel() -> anyhow::Result<()> {
        let test_path = "test_report.xlsx";
        let _ = std::fs::remove_file(test_path);

        let solves = vec![generate_simulated_solve(), generate_simulated_solve()];

        export_session_to_excel(test_path, &solves)?;

        assert!(std::path::Path::new(test_path).exists(), "El archivo Excel debe existir");

        std::fs::remove_file(test_path)?;
        assert!(!std::path::Path::new(test_path).exists(), "El archivo Excel debe ser borrado al finalizar");

        Ok(())
    }
}
