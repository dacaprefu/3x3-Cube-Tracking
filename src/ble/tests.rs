use crate::ble::clock_sync::ClockSync;
use crate::ble::crypto::{derive_key_iv, extract_mac_from_properties, GanCipher};
use crate::ble::protocol::{
    decode_packet, encode_packet, GanEvent, GanFaceletsEvent, GanGyroEvent, GanMoveEvent,
    MoveDirection,
};
use crate::cube::color::Face;
use crate::cube::moves::Move;
use crate::cube::state::Cube;
use btleplug::api::{BDAddr, PeripheralProperties};
use glam::{Quat, Vec3};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[test]
fn test_mac_derivation_and_cipher() {
    let mac: [u8; 6] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    let (key, iv) = derive_key_iv(mac);

    assert_eq!(key.len(), 16);
    assert_eq!(iv.len(), 16);

    let cipher = GanCipher::new(mac);

    let plaintext = b"Hello, GAN Gen4!";
    let ciphertext = cipher.encrypt_cbc(plaintext).expect("Encryption failed");
    assert_eq!(ciphertext.len() % 16, 0);

    let decrypted = cipher.decrypt_cbc(&ciphertext).expect("Decryption failed");
    assert_eq!(&decrypted[..plaintext.len()], plaintext);
}

#[test]
fn test_mac_extraction_from_properties() {
    let mut props = PeripheralProperties::default();
    props.address = BDAddr::from([0u8; 6]);

    let mut mfg_data = HashMap::new();
    let expected_mac: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
    mfg_data.insert(0x0812, expected_mac.to_vec());
    props.manufacturer_data = mfg_data;

    let extracted = extract_mac_from_properties(&props).expect("Extraction should succeed");
    assert_eq!(extracted, expected_mac);
}

#[test]
fn test_decode_encrypted_move_packet() {
    let mac: [u8; 6] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
    let cipher = GanCipher::new(mac);

    let move_event = GanEvent::Move(GanMoveEvent {
        face: Face::R,
        direction: MoveDirection::CounterClockwise,
        hardware_timestamp_ms: 45678,
        sequence_number: 42,
    });

    let raw_packet = encode_packet(&move_event);
    let encrypted = cipher.encrypt_cbc(&raw_packet).expect("Encryption failed");

    // Simular recepción y desencriptación
    let decrypted = cipher.decrypt_cbc(&encrypted).expect("Decryption failed");
    let decoded = decode_packet(&decrypted).expect("Decoding failed");

    match decoded {
        GanEvent::Move(m) => {
            assert_eq!(m.face, Face::R);
            assert_eq!(m.direction, MoveDirection::CounterClockwise);
            assert_eq!(m.hardware_timestamp_ms, 45678);
            assert_eq!(m.sequence_number, 42);
            assert_eq!(m.to_cube_move(), Move::RPrime);
        }
        _ => panic!("Esperado evento de movimiento"),
    }
}

#[test]
fn test_decode_encrypted_facelets_and_sync_check() {
    let mac: [u8; 6] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
    let cipher = GanCipher::new(mac);

    let cube = Cube::new();
    let stickers = cube.to_stickers();

    let facelets_event = GanEvent::Facelets(GanFaceletsEvent {
        hardware_timestamp_ms: 12345,
        facelets: stickers,
    });

    let raw_packet = encode_packet(&facelets_event);
    let encrypted = cipher.encrypt_cbc(&raw_packet).expect("Encryption failed");

    let decrypted = cipher.decrypt_cbc(&encrypted).expect("Decryption failed");
    let decoded = decode_packet(&decrypted).expect("Decoding failed");

    match decoded {
        GanEvent::Facelets(f) => {
            assert_eq!(f.hardware_timestamp_ms, 12345);
            // Debe estar sincronizado con el cubo resuelto
            assert!(f.verify_matches_cube(&cube));

            // Si giramos el cubo interno, debe detectar desincronización
            let mut desynced_cube = cube.clone();
            desynced_cube.apply_move(Move::R);
            assert!(!f.verify_matches_cube(&desynced_cube));
        }
        _ => panic!("Esperado evento de facelets"),
    }
}

#[test]
fn test_decode_encrypted_gyro_packet() {
    let mac: [u8; 6] = [0x55, 0x66, 0x77, 0x88, 0x99, 0xAA];
    let cipher = GanCipher::new(mac);

    let quat = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let vel = Vec3::new(1.2, -0.5, 3.4);

    let gyro_event = GanEvent::Gyro(GanGyroEvent {
        hardware_timestamp_ms: 99999,
        quaternion: quat,
        angular_velocity: vel,
    });

    let raw_packet = encode_packet(&gyro_event);
    let encrypted = cipher.encrypt_cbc(&raw_packet).expect("Encryption failed");

    let decrypted = cipher.decrypt_cbc(&encrypted).expect("Decryption failed");
    let decoded = decode_packet(&decrypted).expect("Decoding failed");

    match decoded {
        GanEvent::Gyro(g) => {
            assert_eq!(g.hardware_timestamp_ms, 99999);
            // Tolerancia por cuantización en 16 bits
            assert!((g.quaternion.dot(quat).abs() - 1.0).abs() < 0.01);
            assert!((g.angular_velocity.x - 1.2).abs() < 0.05);
            assert!((g.angular_velocity.y - (-0.5)).abs() < 0.05);
            assert!((g.angular_velocity.z - 3.4).abs() < 0.05);
        }
        _ => panic!("Esperado evento de giroscopio"),
    }
}

#[test]
fn test_clock_synchronization() {
    let mut sync = ClockSync::new();
    let t0_pc = Instant::now();

    // Paquete 1 del cubo en t=1000ms
    let a1 = sync.synchronize(1000, t0_pc);
    assert_eq!(a1.cube_ms, 1000);
    assert_eq!(a1.elapsed_solve_ms, 0);

    // Paquete 2 del cubo en t=1500ms (500ms de tiempo hardware del cubo)
    // Supongamos que por jitter de Bluetooth llegó 550ms después en el PC
    let t1_pc = t0_pc + Duration::from_millis(550);
    let a2 = sync.synchronize(1500, t1_pc);

    assert_eq!(a2.elapsed_solve_ms, 500);
    // El instante alineado debe basarse exactamente en los 500ms del cubo
    assert_eq!(a2.pc_instant, t0_pc + Duration::from_millis(500));
    assert_eq!(a2.estimated_jitter_ms, 50); // 550 - 500 = 50ms de retraso BLE
}
