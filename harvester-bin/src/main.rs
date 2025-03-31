use std::{
    io::{self, Write},
    time::Instant,
};

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};

use wgpu_sha256_miner::{hash_with_nonce, sha256_parse_words, sha256_preprocess, GpuMiner};

#[tokio::main]
async fn main() -> Result<()> {
    let header_bytes = [0u8; 80];

    // Add padding to reach 128 bytes
    let padded = sha256_preprocess(&header_bytes);
    let mut words = sha256_parse_words(&padded);

    let mut miner = GpuMiner::new(None).await.context("Miner creation failed")?;

    miner.autotune().await;
    println!("Starting mining run...");

    let mut count = 0;
    let winning_nonce: u32;
    let start = Instant::now();

    loop {
        count += miner.get_batch_size();

        let res = miner.run_batch(&words).await.context("Batch run failed.")?;

        if let Some(nonce) = res {
            println!("\nStruck Gold!");
            winning_nonce = nonce;
            break;
        }

        // Print out every 15 loops
        if count % 15 * miner.get_batch_size() == 0 {
            let time = start.elapsed().as_secs_f64();

            let hashes_per_second = ((count as f64) / time) / 1_000_000.0;

            print!("\rTried {} hashes at {:.2} MH/s", count, hashes_per_second);
            io::stdout().flush().unwrap();
        }

        // Timestamp is at byte 68 in the original header
        // 68 / 4 = 7
        words[17] = words[17] + 1;
    }

    // Nonce at 76 / 4 = 19
    words[19] = winning_nonce;

    // Reconstruct the 80-byte header
    let mut header_bytes = [0u8; 80];
    for i in 0..20 {
        let word_bytes = words[i].to_be_bytes(); // Big-endian
        let start = i * 4;
        header_bytes[start..start + 4].copy_from_slice(&word_bytes);
    }

    // Compute and print the hash
    let hash = hash_with_nonce(&header_bytes);
    let hash_hex = hex::encode(hash);
    println!("{}", hash_hex);

    // Convert timestamp bytes to readable format
    let timestamp = u32::from_be_bytes(header_bytes[68..72].try_into().unwrap());
    let datetime = Utc.timestamp_opt(timestamp as i64, 0).unwrap();

    println!("Nonce: {}\nTimestamp: {}", winning_nonce, datetime);

    Ok(())
}
