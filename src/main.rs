use chrono::{DateTime, Utc};
use clickhouse::Client;
use rand::Rng;
use std::time::Instant;
use tracing::error;

const CLICKHOUSE_URL: &str = "http://localhost:8123";
const CLICKHOUSE_DATABASE: &str = "mining";
const CLICKHOUSE_USERNAME: &str = "default";
const CLICKHOUSE_PASSWORD: &str = "5555";
const BATCH_SIZE: usize = 1000;
const TOTAL_SHARES: usize = 1000000;

#[derive(Debug, Clone)]
struct ShareLog {
    timestamp: DateTime<Utc>,
    channel_id: u32,
    sequence_number: u32,
    job_id: u32,
    nonce: u32,
    ntime: u32,
    version: u32,
    hash: String,
    share_status: u8,
    extranonce: String,
    difficulty: f64,
}

fn generate_fake_share(sequence_number: u32) -> ShareLog {
    let mut rng = rand::thread_rng();
    
    let difficulty = rng.gen_range(1000.0..2000.0);
    
    let mut hash = vec![0u8; 32];
    rng.fill(&mut hash[..]);
    
    let mut extranonce = vec![0u8; 16];
    rng.fill(&mut extranonce[..]);
    // генерим шары с равномерным распределением на неделю назад и неделю вперед от сейчас
    let now = Utc::now();
    let two_weeks = chrono::Duration::days(14);
    let random_seconds = rng.gen_range(0..two_weeks.num_seconds());

    let timestamp = now - chrono::Duration::days(7) + chrono::Duration::seconds(random_seconds);

    ShareLog {
        timestamp,
        channel_id: rng.gen_range(1..=10),
        sequence_number,
        job_id: rng.gen(),
        nonce: rng.gen(),
        ntime: timestamp.timestamp() as u32,
        version: 536870912,
        hash: hex::encode(&hash),
        share_status: rng.gen_range(0..=3),
        extranonce: hex::encode(&extranonce),
        difficulty: difficulty,
    }
}

async fn initialize_table(client: &Client) -> Result<(), clickhouse::error::Error> {
    client
        .query(
            "CREATE TABLE IF NOT EXISTS shares (
                channel_id UInt32,
                sequence_number UInt32,
                job_id UInt32,
                nonce UInt32,
                ntime UInt32,
                version UInt32,
                hash String,
                share_status UInt8,
                extranonce String,
                difficulty Float64,
                timestamp DateTime64(3) DEFAULT now64(3)
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMMDD(timestamp)
            ORDER BY (channel_id, timestamp, sequence_number)
            SETTINGS index_granularity = 8192",
        )
        .execute()
        .await?;

    client
        .query(
            "CREATE MATERIALIZED VIEW IF NOT EXISTS mv_hash_rate_stats
            ENGINE = SummingMergeTree()
            PARTITION BY toYYYYMMDD(period_start)
            ORDER BY (channel_id, period_start)
            AS
            SELECT
                channel_id,
                toStartOfMinute(timestamp) as period_start,
                count() as share_count,
                sum(difficulty * pow(2, 32)) as total_hashes,
                min(timestamp) as min_timestamp,
                max(timestamp) as max_timestamp
            FROM shares
            GROUP BY channel_id, period_start",
        )
        .execute()
        .await
}

async fn write_batch(client: &Client, batch: &[ShareLog]) -> Result<(), clickhouse::error::Error> {
    let mut values = Vec::with_capacity(batch.len());
    for log in batch {
        values.push(format!(
            "({}, {}, {}, {}, {}, {}, '{}', {}, '{}', {}, '{}')",
            log.channel_id,
            log.sequence_number,
            log.job_id,
            log.nonce,
            log.ntime,
            log.version,
            log.hash,
            log.share_status,
            log.extranonce,
            log.difficulty,
            log.timestamp.format("%Y-%m-%d %H:%M:%S.%3f")
        ));
    }

    let query = format!(
        "INSERT INTO shares (
            channel_id, sequence_number, job_id, nonce,
            ntime, version, hash, share_status,
            extranonce, difficulty, timestamp
        ) VALUES {}",
        values.join(",")
    );

    client.query(&query).execute().await
}

#[tokio::main]
async fn main() {
    let start_time = Instant::now();
    
    println!("Starting share faker...");
    println!("Batch size: {}", BATCH_SIZE);
    println!("Total shares to generate: {}", TOTAL_SHARES);

    let client = Client::default()
        .with_url(CLICKHOUSE_URL)
        .with_database(CLICKHOUSE_DATABASE)
        .with_user(CLICKHOUSE_USERNAME)
        .with_password(CLICKHOUSE_PASSWORD);

    if let Err(e) = initialize_table(&client).await {
        error!("Failed to initialize table: {}", e);
        return;
    }

    let mut total_written = 0;
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    
    for i in 0..TOTAL_SHARES {
        batch.push(generate_fake_share(i as u32));

        if batch.len() >= BATCH_SIZE {
            if let Err(e) = write_batch(&client, &batch).await {
                error!("Error writing batch: {}", e);
            } else {
                total_written += batch.len();
                let elapsed = start_time.elapsed();
                println!(
                    "Wrote batch of {} shares. Total written: {}. Time elapsed: {}.{:06} seconds",
                    batch.len(),
                    total_written,
                    elapsed.as_secs(),
                    elapsed.subsec_micros()
                );
            }
            batch.clear();
        }
    }

    if !batch.is_empty() {
        if let Err(e) = write_batch(&client, &batch).await {
            error!("Error writing final batch: {}", e);
        } else {
            total_written += batch.len();
            let elapsed = start_time.elapsed();
            println!(
                "Wrote final batch of {} shares. Total written: {}. Time elapsed: {}.{:06} seconds",
                batch.len(),
                total_written,
                elapsed.as_secs(),
                elapsed.subsec_micros()
            );
        }
    }

    let duration = start_time.elapsed();
    println!("\nExecution completed:");
    println!("Total shares written: {}", total_written);
    println!(
        "Time taken: {}.{:06} seconds",
        duration.as_secs(),
        duration.subsec_micros()
    );
    println!(
        "Average speed: {:.2} shares/second",
        total_written as f64 / (duration.as_secs() as f64 + duration.subsec_micros() as f64 / 1_000_000.0)
    );
}