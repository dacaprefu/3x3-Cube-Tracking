use crate::ble::error::BleError;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use btleplug::api::PeripheralProperties;

pub const GAN_ROOT_KEY: [u8; 16] = [
    0xc6, 0xca, 0x15, 0xdf, 0x4f, 0x6e, 0x13, 0xb6,
    0x77, 0x0d, 0xe6, 0x59, 0x3a, 0xaf, 0xba, 0xa2,
];

pub const GAN_ROOT_IV: [u8; 16] = [
    0x11, 0x23, 0x26, 0x25, 0x86, 0x2a, 0x2c, 0x3b,
    0x55, 0x06, 0x7f, 0x31, 0x7e, 0x67, 0x21, 0x57,
];

/// Extrae la dirección MAC (6 bytes) desde las propiedades del periférico BLE.
/// Soporta manufacturer data (Windows / macOS) y dirección directa del adaptador (Linux).
pub fn extract_mac_from_properties(props: &PeripheralProperties) -> Result<[u8; 6], BleError> {
    // 1. Revisar dirección directa del adaptador (típico en Linux)
    let addr_bytes = props.address.into_inner();
    if addr_bytes != [0u8; 6] {
        return Ok(addr_bytes);
    }

    // 2. Revisar manufacturer data (típico en Windows y macOS donde la dirección BLE se oculta)
    for (_mfg_id, data) in &props.manufacturer_data {
        // En cubos GAN, el manufacturer data suele incluir la dirección MAC (6 bytes)
        // ya sea al inicio, tras un prefijo de 2 bytes, o al final
        if data.len() >= 6 {
            // Si tiene exactamente 6 bytes o los últimos 6 bytes parecen una MAC válida
            if data.len() == 6 {
                let mut mac = [0u8; 6];
                mac.copy_from_slice(data);
                return Ok(mac);
            }
            // Si data tiene más de 6 bytes, buscar subsecuencia no nula de 6 bytes
            let mut mac = [0u8; 6];
            mac.copy_from_slice(&data[data.len() - 6..]);
            if mac != [0u8; 6] {
                return Ok(mac);
            }
            // Probar los primeros 6 bytes
            mac.copy_from_slice(&data[..6]);
            if mac != [0u8; 6] {
                return Ok(mac);
            }
        }
    }

    Err(BleError::MacExtractionFailed)
}

/// Deriva la clave y el IV AES-128 para el protocolo GAN Gen4 a partir de la dirección MAC.
pub fn derive_key_iv(mac: [u8; 6]) -> ([u8; 16], [u8; 16]) {
    let mut key = [0u8; 16];
    let mut iv = [0u8; 16];

    for i in 0..16 {
        key[i] = GAN_ROOT_KEY[i].wrapping_add(mac[5 - (i % 6)]);
        iv[i] = GAN_ROOT_IV[i].wrapping_add(mac[i % 6]);
    }

    (key, iv)
}

#[derive(Debug, Clone)]
pub struct GanCipher {
    key: [u8; 16],
    iv: [u8; 16],
}

impl GanCipher {
    pub fn new(mac: [u8; 6]) -> Self {
        let (key, iv) = derive_key_iv(mac);
        Self { key, iv }
    }

    pub fn from_key_iv(key: [u8; 16], iv: [u8; 16]) -> Self {
        Self { key, iv }
    }

    pub fn key(&self) -> &[u8; 16] {
        &self.key
    }

    pub fn iv(&self) -> &[u8; 16] {
        &self.iv
    }

    /// Desencripta un bloque o secuencia de bloques con AES-128 en modo CBC.
    pub fn decrypt_cbc(&self, ciphertext: &[u8]) -> Result<Vec<u8>, BleError> {
        if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
            return Err(BleError::DecryptionFailed(format!(
                "Longitud de ciphertext ({}) no es múltiplo de 16 bytes",
                ciphertext.len()
            )));
        }

        let cipher = Aes128::new(GenericArray::from_slice(&self.key));
        let mut plaintext = Vec::with_capacity(ciphertext.len());
        let mut prev_block = self.iv;

        for chunk in ciphertext.chunks(16) {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.decrypt_block(&mut block);

            let mut decrypted_chunk = [0u8; 16];
            for j in 0..16 {
                decrypted_chunk[j] = block[j] ^ prev_block[j];
            }
            plaintext.extend_from_slice(&decrypted_chunk);

            prev_block.copy_from_slice(chunk);
        }

        Ok(plaintext)
    }

    /// Encripta con AES-128 en modo CBC.
    pub fn encrypt_cbc(&self, plaintext: &[u8]) -> Result<Vec<u8>, BleError> {
        let mut padded = plaintext.to_vec();
        let remainder = padded.len() % 16;
        if remainder != 0 {
            let pad_len = 16 - remainder;
            padded.extend(std::iter::repeat(0u8).take(pad_len));
        }

        let cipher = Aes128::new(GenericArray::from_slice(&self.key));
        let mut ciphertext = Vec::with_capacity(padded.len());
        let mut prev_block = self.iv;

        for chunk in padded.chunks(16) {
            let mut block_bytes = [0u8; 16];
            for j in 0..16 {
                block_bytes[j] = chunk[j] ^ prev_block[j];
            }

            let mut block = GenericArray::clone_from_slice(&block_bytes);
            cipher.encrypt_block(&mut block);

            let encrypted_slice = block.as_slice();
            ciphertext.extend_from_slice(encrypted_slice);
            prev_block.copy_from_slice(encrypted_slice);
        }

        Ok(ciphertext)
    }

    /// Desencripta con AES-128 en modo ECB.
    pub fn decrypt_ecb(&self, ciphertext: &[u8]) -> Result<Vec<u8>, BleError> {
        if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
            return Err(BleError::DecryptionFailed(format!(
                "Longitud de ciphertext ({}) no es múltiplo de 16 bytes",
                ciphertext.len()
            )));
        }

        let cipher = Aes128::new(GenericArray::from_slice(&self.key));
        let mut plaintext = Vec::with_capacity(ciphertext.len());

        for chunk in ciphertext.chunks(16) {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.decrypt_block(&mut block);
            plaintext.extend_from_slice(block.as_slice());
        }

        Ok(plaintext)
    }

    /// Encripta con AES-128 en modo ECB.
    pub fn encrypt_ecb(&self, plaintext: &[u8]) -> Result<Vec<u8>, BleError> {
        let mut padded = plaintext.to_vec();
        let remainder = padded.len() % 16;
        if remainder != 0 {
            let pad_len = 16 - remainder;
            padded.extend(std::iter::repeat(0u8).take(pad_len));
        }

        let cipher = Aes128::new(GenericArray::from_slice(&self.key));
        let mut ciphertext = Vec::with_capacity(padded.len());

        for chunk in padded.chunks(16) {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.encrypt_block(&mut block);
            ciphertext.extend_from_slice(block.as_slice());
        }

        Ok(ciphertext)
    }
}
