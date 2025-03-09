use std::{convert::TryInto, u8};

use hex::FromHexError;
use sha2::{Digest, Sha256};

pub struct BlockHeader {
    version: u32,
    prev_hash: [u8; 32],
    merkle_root: [u8; 32],
    timestamp: u32,
    bits: u32,
} 

/// Holds block header information
impl BlockHeader {
    pub fn new(
        version: &str,
        prev_hash: &str,
        merkle_root: &str,
        timestamp: &str,
        bits: &str
    ) -> Result<Self, FromHexError> {
            let version = Self::hex_to_u32(version)?;
            let prev_hash = Self::hex_to_32_bytes(prev_hash)?;
            let merkle_root = Self::hex_to_32_bytes(merkle_root)?;
            let timestamp = Self::hex_to_u32(timestamp)?;
            let bits =  Self::hex_to_u32(bits)?;

            Ok(
                BlockHeader {version, prev_hash, merkle_root, timestamp, bits}
            )
    }

    /// Converts the header fields and returns them as an [u8; 76]
    pub fn to_bytes(&self) -> [u8; 76] {
            let mut fixed_bytes = [0u8; 76];
            fixed_bytes[0..4].copy_from_slice(&self.version.to_le_bytes());
            fixed_bytes[4..36].copy_from_slice(&self.prev_hash);
            fixed_bytes[36..68].copy_from_slice(&self.merkle_root);
            fixed_bytes[68..72].copy_from_slice(&self.timestamp.to_le_bytes());
            fixed_bytes[72..76].copy_from_slice(&self.bits.to_le_bytes());

            fixed_bytes
    }
    
    // Helper functions
    fn hex_to_u32(s: &str) -> Result<u32, FromHexError> {
        // 4 bytes = 32 bits
        let bytes: [u8; 4] = hex::decode(s)?
            .try_into()
            .map_err(|_| FromHexError::InvalidStringLength)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn hex_to_32_bytes(s: &str) -> Result<[u8; 32], FromHexError> {
        hex::decode(s)?
            .try_into()
            .map_err(|_| FromHexError::InvalidStringLength)
    }
}

pub fn hash_with_nonce(header: &[u8; 80])-> [u8; 32] {
    Sha256::digest(Sha256::digest(header)).into()
}

/// Adds padding to the header to make it 128 bytes (1024 bits)
pub fn sha256_preprocess(header: &[u8; 80]) -> [u8; 128] {
    // Initialize to 128 bytes of 0
    let mut padded = [0u8; 128];
    // First 80 bytes are from the original header
    padded[0..80].copy_from_slice(header);
    
    // Add a byte with 1
    padded[80] = 0x80;

    // Add the length 640 which fits in 2 bytes
    padded[126] = 0x02;
    padded[127] = 0x80;

    padded
}

/// Parse the 32x32-bit words, expects 128 byte header
pub fn sha256_parse_words(header: &[u8; 128]) -> [u32; 32] {
    let mut words = [0u32; 32];
    // Words are chunks of 4 byte = 32 bit
    for (i, chunk) in header.chunks_exact(4).enumerate() {
        words[i] = u32::from_be_bytes(chunk.try_into().unwrap());
    }
    words
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn preprocess_header_is_copied_correctly() {
        let header = [0x01; 80];
        let padded = sha256_preprocess(&header);
        assert_eq!(&padded[0..80], &header);
    }

    #[test]
    fn preprocess_padding_byte_is_set() {
        let header = [0x00; 80];
        let padded = sha256_preprocess(&header);
        assert_eq!(padded[80], 0x80);
    }

    #[test]
    fn preprocess_zero_padding_is_correct() {
        let header = [0xFF; 80];
        let padded = sha256_preprocess(&header);
        for i in 81..120 {
            assert_eq!(padded[i], 0x00);
        }
    }

    #[test]
    fn preprocess_length_field_is_correct() {
        let header = [0x00; 80];
        let padded = sha256_preprocess(&header);
        let expected_length = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x80];
        assert_eq!(&padded[120..128], &expected_length);
    }

    #[test]
    fn preprocess_full_padded_output() {
        let header = [0x01; 80];
        let mut expected = [0x00; 128];
        expected[0..80].copy_from_slice(&header);
        expected[80] = 0x80;
        let length_bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x80];
        expected[120..128].copy_from_slice(&length_bytes);
        let padded = sha256_preprocess(&header);
        assert_eq!(padded, expected);
    }
}
