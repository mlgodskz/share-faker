use chrono::{DateTime, Utc};
use clickhouse::Client;
use primitive_types::U256;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error};

// Constants
const CLICKHOUSE_URL: &str = "http://localhost:8123";
const CLICKHOUSE_DATABASE: &str = "mining";
const CLICKHOUSE_USERNAME: &str = "default";
const CLICKHOUSE_PASSWORD: &str = "5555";
const BATCH_SIZE: usize = 10;
const TOTAL_SHARES: usize = 100;
const TARGET_HEX: &str = "00000000000010c6e6d9be4cd700000000000000000000000000000000000000";
const MAX_TARGET_HEX: &str = "00000000FFFF0000000000000000000000000000000000000000000000000000";

lazy_static::lazy_static! {
    static ref MAX_TARGET: U256 = U256::from_str_radix(MAX_TARGET_HEX, 16).unwrap();
}

#[derive(Debug, Clone)]
struct ShareLog {
    timestamp: DateTime<Utc>,
    channel_id: u32,
    sequence_number: u32,
    job_id: u32,
    nonce: u32,
    ntime: u32,
    version: u32,
    target: Vec<u8>,
    extranonce: Option<Vec<u8>>,
    is_valid: bool,
    error_code: Option<String>,
    hash: Vec<u8>,
    difficulty: f64,
}

fn calculate_difficulty_from_hash(target: &[u8]) -> f64 {
    let current_target = U256::from_big_endian(target);

    let (numerator, denominator, needs_inversion) = if current_target > *MAX_TARGET {
        (current_target, *MAX_TARGET, true)
    } else {
        (*MAX_TARGET, current_target, false)
    };

    let shift_amount = numerator.bits().max(denominator.bits()).saturating_sub(53);

    let ratio =
        (numerator >> shift_amount).as_u64() as f64 / (denominator >> shift_amount).as_u64() as f64;

    if needs_inversion {
        1.0 / ratio
    } else {
        ratio
    }
}

fn generate_fake_share(sequence_number: u32) -> ShareLog {
    let mut rng = rand::thread_rng();
    
    // Generate target from constant
    let target = hex::decode(TARGET_HEX).unwrap();
    
    // Generate random hash that would be valid for the target
    let mut hash = vec![0u8; 32];
    rng.fill(&mut hash[..]);
    
    // Get current Unix timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;

    ShareLog {
        timestamp: Utc::now(),
        channel_id: rng.gen_range(1..=10),
        sequence_number,
        job_id: rng.gen(),
        nonce: rng.gen(),
        ntime: now,
        version: 536870912, // Common version for Bitcoin
        target: target.clone(),
        extranonce: Some(vec![1, 2, 3, 4]),
        is_valid: true,
        error_code: None,
        hash: hash.clone(),
        difficulty: calculate_difficulty_from_hash(&hash),
    }
}

async fn initialize_table(client: &Client) -> Result<(), clickhouse::error::Error> {
    client
        .query(
            "CREATE TABLE IF NOT EXISTS fake_share_logs (
                timestamp DateTime,
                channel_id UInt32,
                sequence_number UInt32,
                job_id UInt32,
                nonce UInt32,
                ntime UInt32,
                version UInt32,
                target Array(UInt8),
                extranonce Array(UInt8),
                is_valid UInt8,
                error_code Nullable(String),
                hash Array(UInt8),
                difficulty Float64,
                _timestamp_minute DateTime MATERIALIZED toStartOfMinute(timestamp),
                _timestamp_hour DateTime MATERIALIZED toStartOfHour(timestamp)
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMMDD(timestamp)
            PRIMARY KEY (channel_id, timestamp)
            ORDER BY (channel_id, timestamp, sequence_number)",
        )
        .execute()
        .await
}

async fn write_batch(client: &Client, batch: &[ShareLog]) -> Result<(), clickhouse::error::Error> {
    let mut values = Vec::with_capacity(batch.len());
    for log in batch {
        let extranonce_str = log
            .extranonce
            .as_ref()
            .map(|v| format!("[{}]", v.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",")))
            .unwrap_or_else(|| "[]".to_string());
        let target_str = format!(
            "[{}]",
            log.target
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let hash_str = format!(
            "[{}]",
            log.hash
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        values.push(format!(
            "('{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
            log.timestamp.format("%Y-%m-%d %H:%M:%S"),
            log.channel_id,
            log.sequence_number,
            log.job_id,
            log.nonce,
            log.ntime,
            log.version,
            target_str,
            extranonce_str,
            if log.is_valid { 1 } else { 0 },
            log.error_code
                .as_ref()
                .map(|s| format!("'{}'", s))
                .unwrap_or_else(|| "NULL".to_string()),
            hash_str,
            log.difficulty
        ));
    }

    
    let query = format!(
        "INSERT INTO fake_share_logs (
            timestamp, channel_id, sequence_number, job_id,
            nonce, ntime, version, target, extranonce,
            is_valid, error_code, hash, difficulty
        ) VALUES {}",
        values.join(",")
    );

    client.query(&query).execute().await
}

#[tokio::main]
async fn main() {
    let client = Client::default()
        .with_url(CLICKHOUSE_URL)
        .with_database(CLICKHOUSE_DATABASE)
        .with_user(CLICKHOUSE_USERNAME)
        .with_password(CLICKHOUSE_PASSWORD);

    if let Err(e) = initialize_table(&client).await {
        error!("Failed to initialize table: {}", e);
        return;
    }

    let mut batch = Vec::with_capacity(BATCH_SIZE);
    for i in 0..TOTAL_SHARES {
        batch.push(generate_fake_share(i as u32));

        if batch.len() >= BATCH_SIZE {
            if let Err(e) = write_batch(&client, &batch).await {
                error!("Error writing batch: {}", e);
            }
            batch.clear();
        }
    }

    if !batch.is_empty() {
        if let Err(e) = write_batch(&client, &batch).await {
            error!("Error writing final batch: {}", e);
        }
    }
}