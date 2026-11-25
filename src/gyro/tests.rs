use crate::cube::color::Face;
use crate::cube::moves::Move;
use crate::gyro::regrip::RegripDetector;
use crate::gyro::rotation::{CubeOrientation, GyroProcessor};
use glam::{Quat, Vec3};
use std::f32::consts::FRAC_PI_2;

#[test]
fn test_gyro_rotation_y_and_y_prime() {
    let mut processor = GyroProcessor::new();
    assert_eq!(processor.current_orientation(), CubeOrientation::IDENTITY);

    // 1. Simular rotación 'y' (giro de 90° alrededor del eje Y)
    // Quaternion de rotación de 90° en sentido horario mirando desde arriba
    let q_y = Quat::from_axis_angle(Vec3::Y, -FRAC_PI_2);

    let detected = processor.process_quaternion(q_y, 1000);
    assert!(detected.is_some(), "Debe detectar rotación Y");
    let rot_event = detected.unwrap();
    assert_eq!(rot_event.rotation, Move::Y);
    assert_eq!(rot_event.new_orientation.up, Face::U);
    assert_eq!(rot_event.new_orientation.front, Face::R);
    assert_eq!(processor.current_orientation().front, Face::R);

    // 2. Simular rotación 'y'' (giro de vuelta de -90° regresando a la posición inicial)
    let q_y_prime = Quat::IDENTITY;
    let detected_back = processor.process_quaternion(q_y_prime, 1500);
    assert!(detected_back.is_some(), "Debe detectar rotación Y'");
    let rot_back = detected_back.unwrap();
    assert_eq!(rot_back.rotation, Move::YPrime);
    assert_eq!(rot_back.new_orientation, CubeOrientation::IDENTITY);
}

#[test]
fn test_gyro_rotation_x_and_z() {
    let mut processor = GyroProcessor::new();

    // Rotación 'x' (+90° pitch)
    let q_x = Quat::from_axis_angle(Vec3::X, FRAC_PI_2);
    let detected_x = processor.process_quaternion(q_x, 2000);
    assert!(detected_x.is_some());
    assert_eq!(detected_x.unwrap().rotation, Move::X);
    assert_eq!(processor.current_orientation().up, Face::F);
    assert_eq!(processor.current_orientation().front, Face::D);

    // Regresar a identidad
    processor.reset_orientation(CubeOrientation::IDENTITY);

    // Rotación 'z' (+90° roll)
    let q_z = Quat::from_axis_angle(Vec3::Z, -FRAC_PI_2);
    let detected_z = processor.process_quaternion(q_z, 3000);
    assert!(detected_z.is_some());
    assert_eq!(detected_z.unwrap().rotation, Move::Z);
    assert_eq!(processor.current_orientation().up, Face::L);
    assert_eq!(processor.current_orientation().front, Face::F);
}

#[test]
fn test_perspective_move_translation() {
    // Cuando el cubo está rotado con 'y', la cara que el usuario ve de frente es la cara R del cubo
    let orientation = CubeOrientation::IDENTITY.apply_rotation(Move::Y);
    assert_eq!(orientation.front, Face::R);
    assert_eq!(orientation.right(), Face::B);
    assert_eq!(orientation.up, Face::U);

    // Si el usuario gira la cara Frontal frente a sus ojos, en el cubo interno es la cara R!
    let cube_move = orientation.perspective_to_cube_move(Move::F);
    assert_eq!(cube_move, Move::R);

    let cube_move_prime = orientation.perspective_to_cube_move(Move::FPrime);
    assert_eq!(cube_move_prime, Move::RPrime);

    // Si gira la cara Derecha (Right), en el cubo interno es Back (B)!
    let cube_move_right = orientation.perspective_to_cube_move(Move::R);
    assert_eq!(cube_move_right, Move::B);

    // Si gira Up (U), sigue siendo U
    let cube_move_up = orientation.perspective_to_cube_move(Move::U);
    assert_eq!(cube_move_up, Move::U);
}

#[test]
fn test_regrip_detection_and_invalidation_on_move() {
    let mut detector = RegripDetector::new();

    // 1. Simular un regrip genuino: micro-inclinación de 15° con velocidad angular alta,
    // seguida de estabilización sin giro de capa.
    let tilt_angle_rad = 15.0f32.to_radians();
    let q_tilted = Quat::from_axis_angle(Vec3::X, tilt_angle_rad);
    let high_vel = Vec3::new(2.0, 0.0, 0.0);
    let low_vel = Vec3::new(0.1, 0.0, 0.0);

    // Inicio de la inclinación
    let e1 = detector.on_gyro(100, q_tilted, high_vel);
    assert!(e1.is_none());

    // Se mantiene inclinada
    let e2 = detector.on_gyro(150, q_tilted, high_vel);
    assert!(e2.is_none());

    // Se estabiliza en reposo a los 220ms
    let e3 = detector.on_gyro(220, q_tilted, low_vel);
    assert!(e3.is_some(), "Debe emitirse un evento de Regrip al estabilizarse");
    let regrip = e3.unwrap();
    assert_eq!(regrip.duration_ms, 120);
    assert!((regrip.peak_angle_deg - 15.0).abs() < 1.0);
    assert_eq!(detector.regrip_count(), 1);

    // 2. Simular otra micro-inclinación pero con un giro de capa en medio:
    // NO debe detectarse como regrip porque el usuario giró una capa.
    detector.on_gyro(300, q_tilted, high_vel);
    detector.on_move(350); // Giro de capa
    let e_after_move = detector.on_gyro(400, q_tilted, low_vel);
    assert!(e_after_move.is_none(), "No debe emitirse regrip si hubo un giro de capa");
    assert_eq!(detector.regrip_count(), 1);
}
