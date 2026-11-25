use crate::analytics::analyzer::{phase_str, SolveAnalyzer};
use crate::cfop::phase::CfopPhase;
use crate::cfop::tracker::CfopTracker;
use crate::cube::moves::{parse_moves, Move};

#[test]
fn test_solve_analyzer_with_anomalies_and_exact_tags() {
    let mut tracker = CfopTracker::new();

    // 1. Scramble base
    let t_perm = parse_moves("R U R' U' R' F R2 U' R' U' R U R' F'").unwrap();
    let sune_inv = parse_moves("R U2 R' U' R U' R'").unwrap();
    let sune = parse_moves("R U R' U R U2 R'").unwrap();
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

    // Cross: D (300ms)
    for m in cross_solve {
        current_time += 300;
        tracker.on_move(m, current_time);
    }
    assert_eq!(tracker.current_phase(), CfopPhase::F2L_1);

    // F2L 1: Generar INEFICIENCIA_F2L con 15 giros (hacer cancelaciones neutras antes de insertar)
    // 10 giros neutros (U U' x 5 = 10 giros) + 5 giros de insert = 15 giros
    for _ in 0..5 {
        current_time += 100;
        tracker.on_move(Move::U, current_time);
        current_time += 100;
        tracker.on_move(Move::UPrime, current_time);
    }
    for &m in &insert {
        current_time += 120;
        tracker.on_move(m, current_time);
    }
    assert_eq!(tracker.current_phase(), CfopPhase::F2L_2);

    // F2L 2: Generar PAUSA_CRITICA de 680ms dentro de F2L-2
    current_time += 140;
    tracker.on_move(insert[0], current_time); // Primer giro de F2L 2

    current_time += 680; // Pausa crítica de exactamente 680ms dentro de F2L 2
    tracker.on_move(insert[1], current_time); // Segundo giro tras la pausa

    for &m in &insert[2..] {
        current_time += 140;
        tracker.on_move(m, current_time);
    }
    assert_eq!(tracker.current_phase(), CfopPhase::F2L_3);

    // F2L 3: Generar BLIND_ROTATION (dos rotaciones seguidas y + y' sin giros de capa intermedios)
    current_time += 200;
    tracker.on_move(Move::Y, current_time);
    current_time += 200;
    tracker.on_move(Move::YPrime, current_time);

    // Giro de capa intermedio para cancelar cualquier rotación consecutiva posterior
    current_time += 100;
    tracker.on_move(Move::U, current_time);
    current_time += 100;
    tracker.on_move(Move::UPrime, current_time);

    // Resolver F2L 3 normalmente
    for &m in &insert {
        current_time += 140;
        tracker.on_move(m, current_time);
    }
    assert_eq!(tracker.current_phase(), CfopPhase::F2L_4);

    // F2L 4: Resolver normalmente
    for &m in &insert {
        current_time += 140;
        tracker.on_move(m, current_time);
    }
    assert_eq!(tracker.current_phase(), CfopPhase::OLL);

    // OLL: Introducir un LOCKUP_OLL de 230ms en medio del algoritmo Sune
    // Movimientos rápidos: Sune[0], Sune[1] a 120ms
    current_time += 200;
    tracker.on_move(sune[0], current_time);
    current_time += 120;
    tracker.on_move(sune[1], current_time);

    // LOCKUP: movimiento 2 tarda 230ms
    current_time += 230;
    tracker.on_move(sune[2], current_time);

    for &m in &sune[3..] {
        current_time += 120;
        tracker.on_move(m, current_time);
    }
    assert_eq!(tracker.current_phase(), CfopPhase::PLL);

    // PLL: Generar DUDA_RECONOCIMIENTO_PLL con 850ms de reconocimiento y doble AUF (U, U')
    current_time += 500;
    tracker.on_move(Move::U, current_time); // Primer AUF (500ms)

    current_time += 230;
    tracker.on_move(Move::UPrime, current_time); // Segundo AUF (230ms)

    // Primer giro efectivo de T-perm tras 120ms (Reconocimiento total = 500 + 230 + 120 = 850ms)
    current_time += 120;
    tracker.on_move(t_perm[0], current_time);

    // Resto de T-perm
    for &m in &t_perm[1..] {
        current_time += 120;
        tracker.on_move(m, current_time);
    }

    assert_eq!(tracker.current_phase(), CfopPhase::Solved);
    assert!(tracker.is_solved());

    // Analizar el solve con SolveAnalyzer
    let analyzer = SolveAnalyzer::new();
    let report = analyzer.analyze(&tracker).expect("El análisis debe ser exitoso");

    // Imprimir anomalías para inspección
    println!("Anomalías detectadas: {:#?}", report.anomalies);

    // Verificaciones exactas solicitadas en la especificación
    assert!(
        report
            .anomalies
            .contains(&"[PAUSA_CRITICA: 680ms en F2L-2]".to_string()),
        "Debe contener [PAUSA_CRITICA: 680ms en F2L-2]"
    );

    assert!(
        report
            .anomalies
            .contains(&"[BLIND_ROTATION: y + y' en F2L-3]".to_string()),
        "Debe contener [BLIND_ROTATION: y + y' en F2L-3]"
    );

    assert!(
        report
            .anomalies
            .contains(&"[INEFICIENCIA_F2L: 15 giros en F2L-1]".to_string()),
        "Debe contener [INEFICIENCIA_F2L: 15 giros en F2L-1]"
    );

    assert!(
        report
            .anomalies
            .contains(&"[LOCKUP_OLL: 230ms]".to_string()),
        "Debe contener [LOCKUP_OLL: 230ms]"
    );

    assert!(
        report
            .anomalies
            .contains(&"[DUDA_RECONOCIMIENTO_PLL: 850ms + doble AUF]".to_string()),
        "Debe contener [DUDA_RECONOCIMIENTO_PLL: 850ms + doble AUF]"
    );

    // Verificar métricas de pausas
    assert!(!report.pauses.is_empty());
    let pause_680 = report.pauses.iter().find(|p| p.duration_ms == 680).unwrap();
    assert_eq!(pause_680.phase, CfopPhase::F2L_2);

    // Verificar TPS bruto vs TPS efectivo
    assert!(report.tps_raw > 0.0);
    assert!(report.tps_effective > report.tps_raw, "TPS efectivo debe ser mayor al descontar pausas muertas");

    // Verificar impuesto de rotaciones
    assert!(report.rotation_tax.total_y_rotations_f2l > 0);
    assert_eq!(report.rotation_tax.blind_rotations.len(), 1);
    assert_eq!(report.rotation_tax.blind_rotations[0].first_rot, Move::Y);
    assert_eq!(report.rotation_tax.blind_rotations[0].second_rot, Move::YPrime);
}

#[test]
fn test_phase_str() {
    assert_eq!(phase_str(CfopPhase::Cross), "Cross");
    assert_eq!(phase_str(CfopPhase::F2L_1), "F2L-1");
    assert_eq!(phase_str(CfopPhase::F2L_2), "F2L-2");
    assert_eq!(phase_str(CfopPhase::F2L_3), "F2L-3");
    assert_eq!(phase_str(CfopPhase::F2L_4), "F2L-4");
    assert_eq!(phase_str(CfopPhase::OLL), "OLL");
    assert_eq!(phase_str(CfopPhase::PLL), "PLL");
}
