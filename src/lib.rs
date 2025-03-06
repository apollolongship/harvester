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
