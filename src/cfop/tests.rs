use crate::cfop::phase::CfopPhase;
use crate::cfop::tracker::CfopTracker;
use crate::cube::moves::{parse_moves, Move};
use crate::cube::state::Cube;

#[test]
fn test_cube_basic_moves_and_solved() {
    let mut cube = Cube::new();
    assert!(cube.is_solved());

    // Sexy move x 6 = solved
    let sexy = parse_moves("R U R' U'").unwrap();
    for _ in 0..6 {
        cube.apply_moves(&sexy);
    }
    assert!(cube.is_solved());

    // Inverses of all moves
    let test_moves = [
        (Move::U, Move::UPrime),
        (Move::D, Move::DPrime),
        (Move::L, Move::LPrime),
        (Move::R, Move::RPrime),
        (Move::F, Move::FPrime),
        (Move::B, Move::BPrime),
        (Move::X, Move::XPrime),
        (Move::Y, Move::YPrime),
        (Move::Z, Move::ZPrime),
    ];

    for (m, m_inv) in test_moves {
        cube.apply_move(m);
        assert!(!cube.is_solved());
        cube.apply_move(m_inv);
        assert!(cube.is_solved());
    }

    // Scramble matching
    let scramble = parse_moves("R U2 F' D L2 B'").unwrap();
    cube.apply_moves(&scramble);
    assert!(cube.matches_scramble(&scramble));
}

#[test]
fn test_stickers_projection_and_reconstruction() {
    let mut cube = Cube::new();
    let stickers_orig = cube.to_stickers();
    assert_eq!(stickers_orig.len(), 54);

    let reconstructed = Cube::from_stickers(&stickers_orig);
    assert_eq!(cube, reconstructed);

    // Apply scramble and test roundtrip
    let scramble = parse_moves("R U R' F2 D' B L2").unwrap();
    cube.apply_moves(&scramble);
    let stickers_scrambled = cube.to_stickers();
    let reconstructed_scrambled = Cube::from_stickers(&stickers_scrambled);
    assert_eq!(cube, reconstructed_scrambled);
}

#[test]
fn test_cfop_normal_solve_with_auf_and_recognition_execution() {
    let mut tracker = CfopTracker::new();

    // Algoritmo T-Perm (PLL, involución: T * T = Identity)
    let t_perm = parse_moves("R U R' U' R' F R2 U' R' U' R U R' F'").unwrap();
    // Algoritmo Sune (OLL) y su inverso
    let sune = parse_moves("R U R' U R U2 R'").unwrap();
    let sune_inv = parse_moves("R U2 R' U' R U' R'").unwrap();

    // Movimientos de desarmado e inserción F2L usando rotaciones y
    let displace = parse_moves("R U R' U'").unwrap();
    let insert = parse_moves("y' U R U' R'").unwrap();

    // Cruz: D' (solve: D)
    let cross_scramble = parse_moves("D'").unwrap();
    let cross_solve = parse_moves("D").unwrap();

    // Construir scramble completo en orden LIFO
    let mut scramble_sequence = Vec::new();
    scramble_sequence.extend_from_slice(&t_perm);
    scramble_sequence.extend_from_slice(&sune_inv);

    // Desarmar los 4 pares F2L
    for _ in 0..4 {
        scramble_sequence.extend_from_slice(&displace);
        scramble_sequence.push(Move::Y);
    }
    scramble_sequence.extend_from_slice(&cross_scramble);

    tracker.apply_scramble(&scramble_sequence);
    assert_eq!(tracker.current_phase(), CfopPhase::Inspection);

    tracker.start_inspection(1000);
    tracker.start_solve(2000);

    let mut current_time = 2000;

    // 1. Resolver Cross (D)
    for m in cross_solve {
        current_time += 300;
        tracker.on_move(m, current_time);
    }
    assert!(tracker.metrics().cross.is_some());
    let cross_metric = tracker.metrics().cross.as_ref().unwrap();
    assert_eq!(cross_metric.phase, CfopPhase::Cross);
    assert_eq!(cross_metric.move_count, 1);
    assert_eq!(cross_metric.duration_ms, 300);
    assert_eq!(tracker.current_phase(), CfopPhase::F2L_1);

    // 2. Resolver F2L 1
    for &m in &insert {
        current_time += 150;
        tracker.on_move(m, current_time);
    }
    assert!(tracker.metrics().f2l_1.is_some());
    assert_eq!(tracker.current_phase(), CfopPhase::F2L_2);

    // 3. Resolver F2L 2
    for &m in &insert {
        current_time += 150;
        tracker.on_move(m, current_time);
    }
    assert!(tracker.metrics().f2l_2.is_some());
    assert_eq!(tracker.current_phase(), CfopPhase::F2L_3);

    // 4. Resolver F2L 3
    for &m in &insert {
        current_time += 150;
        tracker.on_move(m, current_time);
    }
    assert!(tracker.metrics().f2l_3.is_some());
    assert_eq!(tracker.current_phase(), CfopPhase::F2L_4);

    // 5. Resolver F2L 4
    for &m in &insert {
        current_time += 150;
        tracker.on_move(m, current_time);
    }
    assert!(tracker.metrics().f2l_4.is_some());
    assert_eq!(tracker.current_phase(), CfopPhase::OLL);

    // 6. Resolver OLL con pre-AUF (U, U') y algoritmo Sune
    current_time += 400;
    tracker.on_move(Move::U, current_time);

    current_time += 200;
    tracker.on_move(Move::UPrime, current_time);

    // Primer giro efectivo (R de Sune) 300ms después
    current_time += 300;
    tracker.on_move(sune[0], current_time);

    for &m in &sune[1..] {
        current_time += 150;
        tracker.on_move(m, current_time);
    }

    assert!(tracker.metrics().oll.is_some());
    let oll_metric = tracker.metrics().oll.as_ref().unwrap();
    assert_eq!(oll_metric.phase, CfopPhase::OLL);
    assert_eq!(oll_metric.pre_auf, vec![Move::U, Move::UPrime]);
    // Reconocimiento: 400 + 200 + 300 = 900ms
    assert_eq!(oll_metric.recognition_time_ms, Some(900));
    assert_eq!(oll_metric.execution_time_ms, Some(150 * (sune.len() as u64 - 1)));
    assert_eq!(tracker.current_phase(), CfopPhase::PLL);

    // 7. Resolver PLL con pre-AUF (U, U') y T-Perm
    current_time += 500;
    tracker.on_move(Move::U, current_time);

    current_time += 150;
    tracker.on_move(Move::UPrime, current_time);

    // Primer giro efectivo (R de T-perm) 250ms después
    current_time += 250;
    tracker.on_move(t_perm[0], current_time);

    for &m in &t_perm[1..] {
        current_time += 120;
        tracker.on_move(m, current_time);
    }

    assert_eq!(tracker.current_phase(), CfopPhase::Solved);
    assert!(tracker.is_solved());

    let pll_metric = tracker.metrics().pll.as_ref().unwrap();
    assert_eq!(pll_metric.phase, CfopPhase::PLL);
    assert_eq!(pll_metric.pre_auf, vec![Move::U, Move::UPrime]);
    assert_eq!(pll_metric.recognition_time_ms, Some(900));
    assert!(pll_metric.execution_time_ms.unwrap() > 0);
    assert!(tracker.metrics().total_time_ms > 0);
}

#[test]
fn test_x_cross_detection() {
    let mut tracker = CfopTracker::new();

    let cross_scramble = parse_moves("D' F'").unwrap();
    let cross_solve = parse_moves("F D").unwrap();

    tracker.apply_scramble(&cross_scramble);
    tracker.start_inspection(500);

    let mut t = 1000;
    for m in cross_solve {
        t += 300;
        tracker.on_move(m, t);
    }

    let metrics = tracker.metrics();
    assert!(metrics.is_x_cross);
    assert!(metrics.f2l_1.is_some());
    let f2l_1 = metrics.f2l_1.as_ref().unwrap();
    assert_eq!(f2l_1.duration_ms, 0);
    assert_eq!(f2l_1.move_count, 0);
    assert!(f2l_1.is_skip);
}

#[test]
fn test_oll_skip_detection() {
    let mut tracker = CfopTracker::new();

    let f2l_4_scramble = parse_moves("R U R' U'").unwrap();
    let f2l_4_solve = parse_moves("U R U' R'").unwrap();
    let t_perm = parse_moves("R U R' U' R' F R2 U' R' U' R U R' F'").unwrap();

    let mut scramble = Vec::new();
    scramble.extend_from_slice(&t_perm);
    scramble.extend_from_slice(&f2l_4_scramble);

    tracker.apply_scramble(&scramble);
    tracker.start_inspection(100);

    let mut t = 1000;
    for m in f2l_4_solve {
        t += 200;
        tracker.on_move(m, t);
    }

    let metrics = tracker.metrics();
    assert!(metrics.is_oll_skip);
    assert!(metrics.oll.is_some());
    let oll_metric = metrics.oll.as_ref().unwrap();
    assert_eq!(oll_metric.duration_ms, 0);
    assert_eq!(oll_metric.move_count, 0);
    assert!(oll_metric.is_skip);
    assert_eq!(oll_metric.recognition_time_ms, Some(0));
    assert_eq!(oll_metric.execution_time_ms, Some(0));
    assert_eq!(tracker.current_phase(), CfopPhase::PLL);
}

#[test]
fn test_pll_skip_detection() {
    let mut tracker = CfopTracker::new();

    let f2l_4_scramble = parse_moves("R U R' U'").unwrap();
    let f2l_4_solve = parse_moves("U R U' R'").unwrap();

    tracker.apply_scramble(&f2l_4_scramble);
    tracker.start_inspection(100);

    let mut t = 1000;
    for m in f2l_4_solve {
        t += 200;
        tracker.on_move(m, t);
    }

    let metrics = tracker.metrics();
    assert!(metrics.is_oll_skip);
    assert!(metrics.is_pll_skip);
    assert!(metrics.pll.is_some());
    let pll_metric = metrics.pll.as_ref().unwrap();
    assert_eq!(pll_metric.duration_ms, 0);
    assert_eq!(pll_metric.move_count, 0);
    assert!(pll_metric.is_skip);
    assert_eq!(pll_metric.recognition_time_ms, Some(0));
    assert_eq!(pll_metric.execution_time_ms, Some(0));
    assert_eq!(tracker.current_phase(), CfopPhase::Solved);
    assert!(tracker.is_solved());
}
