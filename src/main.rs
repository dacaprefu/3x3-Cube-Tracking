use tracing::{debug, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let is_demo = args.iter().any(|arg| arg == "--demo");

    if is_demo {
        run_demo_simulation().await?;
        return Ok(());
    }

    info!("Iniciando modo normal - Buscando cubo GAN i4 via Bluetooth...");
    println!("Escaneando dispositivos Bluetooth (duración: 5s)...");

    let discovered = cubo_cfop::ble::scan_for_gan_cubes(std::time::Duration::from_secs(5)).await?;
    if discovered.is_empty() {
        eprintln!("No se encontró ningún cubo GAN. Asegúrate de que el cubo esté encendido.");
        println!("Sugerencia: Ejecuta con `cargo run -- --demo` para probar la simulación sin hardware.");
        return Ok(());
    }

    let target = discovered.into_iter().next().unwrap();
    println!("¡Cubo encontrado! Conectando a [{}] (MAC: {:?})...", target.name, target.mac);

    let client = cubo_cfop::ble::GanBleClient::connect(target).await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(cubo_cfop::ble::GanEvent, cubo_cfop::ble::AlignedTimestamp)>(200);

    client.start_event_stream(tx).await?;
    println!("Conexión establecida. Comienza a resolver el cubo. (Presiona Ctrl+C para salir)");

    let mut tracker = cubo_cfop::cfop::CfopTracker::new();
    let mut session_reports = Vec::new();
    let analyzer = cubo_cfop::analytics::SolveAnalyzer::new();

    while let Some((event, aligned)) = rx.recv().await {
        match event {
            cubo_cfop::ble::GanEvent::Move(m_event) => {
                let m = m_event.to_cube_move();
                let ts = aligned.elapsed_solve_ms;
                tracker.on_move(m, ts);

                info!("Movimiento recibido: {} (en t={}ms)", m, ts);

                if tracker.current_phase() == cubo_cfop::cfop::CfopPhase::Solved {
                    println!("\n¡Cubo resuelto con éxito!");
                    match analyzer.analyze(&tracker) {
                        Ok(report) => {
                            println!("=== Reporte del Solve ===");
                            println!("Tiempo Total: {:.2}s", report.total_time_ms as f32 / 1000.0);
                            println!("TPS Efectivo: {:.2}", report.tps_effective);
                            println!("Rotaciones F2L: {}", report.rotation_tax.total_y_rotations_f2l);
                            if !report.anomalies.is_empty() {
                                println!("Anomalías: {:?}", report.anomalies);
                            }

                            session_reports.push(report.clone());

                            let excel_path = "sesion_normal.xlsx";
                            if let Err(e) = cubo_cfop::export::export_session_to_excel(excel_path, &session_reports) {
                                eprintln!("Error exportando sesión a Excel: {}", e);
                            } else {
                                info!("Sesión exportada a {}", excel_path);
                            }

                            dotenvy::dotenv().ok();
                            if std::env::var("GEMINI_API_KEY").is_ok() {
                                println!("Consultando a Gemini AI Coach...");
                                match cubo_cfop::ai_coach::analyze_solve_with_gemini(&report).await {
                                    Ok(diag) => {
                                        println!("\n--- Coach IA ---");
                                        println!("{}", diag);
                                    }
                                    Err(e) => eprintln!("Error en Coach IA: {}", e),
                                }
                            }
                        }
                        Err(e) => eprintln!("Error analizando el solve: {}", e),
                    }

                    tracker = cubo_cfop::cfop::CfopTracker::new();
                    println!("\nListo para el siguiente solve. Resuelve el cubo nuevamente...");
                }
            }
            cubo_cfop::ble::GanEvent::Gyro(g) => {
                debug!("Evento Giroscopio recibido: timestamp={}ms", g.hardware_timestamp_ms);
            }
            cubo_cfop::ble::GanEvent::Facelets(f) => {
                debug!("Evento Facelets recibido: timestamp={}ms", f.hardware_timestamp_ms);
            }
        }
    }

    Ok(())
}

async fn run_demo_simulation() -> anyhow::Result<()> {
    info!("Iniciando simulación de solve CFOP (--demo)...");

    let mut tracker = cubo_cfop::cfop::CfopTracker::new();

    let t_perm = cubo_cfop::cube::parse_moves("R U R' U' R' F R2 U' R' U' R U R' F'").unwrap();
    let sune = cubo_cfop::cube::parse_moves("R U R' U R U2 R'").unwrap();
    let sune_inv = cubo_cfop::cube::parse_moves("R U2 R' U' R U' R'").unwrap();
    let displace = cubo_cfop::cube::parse_moves("R U R' U'").unwrap();
    let insert = cubo_cfop::cube::parse_moves("y' U R U' R'").unwrap(); // Incluye rotación innecesaria en F2L (y')
    let cross_scramble = cubo_cfop::cube::parse_moves("D'").unwrap();
    let cross_solve = cubo_cfop::cube::parse_moves("D").unwrap();

    let mut scramble_sequence = Vec::new();
    scramble_sequence.extend_from_slice(&t_perm);
    scramble_sequence.extend_from_slice(&sune_inv);
    for _ in 0..4 {
        scramble_sequence.extend_from_slice(&displace);
        scramble_sequence.push(cubo_cfop::cube::Move::Y);
    }
    scramble_sequence.extend_from_slice(&cross_scramble);

    tracker.apply_scramble(&scramble_sequence);
    tracker.start_inspection(1000);
    tracker.start_solve(2000);

    let mut current_time = 2000;

    // Cross solve
    for m in cross_solve {
        current_time += 250;
        tracker.on_move(m, current_time);
    }

    // F2L pairs (con una pausa de lookahead >400ms, por ejemplo 550ms, y rotaciones y')
    for (i, pair_moves) in [insert.clone(), insert.clone(), insert.clone(), insert.clone()].into_iter().enumerate() {
        if i == 2 {
            // Pausa de lookahead > 400ms (550ms)
            current_time += 550;
        } else {
            current_time += 200;
        }

        for &m in &pair_moves {
            current_time += 150;
            tracker.on_move(m, current_time);
        }
    }

    // OLL
    current_time += 400; // pausa de reconocimiento OLL
    for &m in &sune {
        current_time += 120;
        tracker.on_move(m, current_time);
    }

    // PLL
    current_time += 450; // pausa de reconocimiento PLL
    for &m in &t_perm {
        current_time += 120;
        tracker.on_move(m, current_time);
    }

    let analyzer = cubo_cfop::analytics::SolveAnalyzer::new();
    let report = analyzer.analyze(&tracker)?;

    let demo_filename = "sesion_demo.xlsx";
    cubo_cfop::export::export_session_to_excel(demo_filename, &[report.clone()])?;
    info!("Reporte exportado exitosamente a {}", demo_filename);

    println!("\n==============================================");
    println!("        RESUMEN DE MÉTRICAS - SOLVE DEMO      ");
    println!("==============================================");
    println!("Tiempo Total: {:.2} s", report.total_time_ms as f32 / 1000.0);
    println!("Tiempo Activo: {:.2} s", report.active_time_ms as f32 / 1000.0);
    println!("Movimientos HTM: {}", report.total_moves_htm);
    println!("TPS Bruto: {:.2}", report.tps_raw);
    println!("TPS Efectivo: {:.2}", report.tps_effective);
    println!("Rotaciones F2L (Impuesto): {}", report.rotation_tax.total_y_rotations_f2l);
    println!("Pausas Críticas (>350ms): {}", report.pauses.iter().filter(|p| p.duration_ms > 350).count());
    
    println!("\nDesglose por Fases:");
    for phase_metric in &report.phases {
        println!(
            "  - {:<6}: {:>3} giros | Duración: {:>5} ms | TPS efec: {:>4.2}",
            cubo_cfop::analytics::phase_str(phase_metric.phase),
            phase_metric.move_count,
            phase_metric.duration_ms,
            phase_metric.tps_effective
        );
    }

    if !report.anomalies.is_empty() {
        println!("\nAnomalías Detectadas:");
        for anomaly in &report.anomalies {
            println!("  * {}", anomaly);
        }
    }
    println!("==============================================\n");

    dotenvy::dotenv().ok();
    if std::env::var("GEMINI_API_KEY").is_ok() {
        info!("Consultando a Gemini AI Coach...");
        match cubo_cfop::ai_coach::analyze_solve_with_gemini(&report).await {
            Ok(diagnosis) => {
                println!("\n=== Diagnóstico del Coach IA (Gemini) ===");
                println!("{}", diagnosis);
                println!("==========================================\n");
            }
            Err(e) => {
                eprintln!("Error al consultar a Gemini AI Coach: {}", e);
            }
        }
    } else {
        println!("Nota: GEMINI_API_KEY no encontrada en .env o entorno. Se omite el diagnóstico del Coach IA.");
    }

    Ok(())
}
