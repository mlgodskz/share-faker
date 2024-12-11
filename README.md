cd /home/ro/projects/nomium/01_SRI-Local-Utils/share-faker
# Запустить генерацию
cargo run

# Удаление всех данных
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "DROP TABLE IF EXISTS mining.mv_hash_rate_stats"

curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "DROP TABLE IF EXISTS mining.shares"

####

## данные за последний час из MV

curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT 
    total_shares,
    total_hashes,
    total_hashes / 3600 AS hashrate
FROM (
    SELECT 
        sum(share_count) AS total_shares, 
        sum(total_hashes) AS total_hashes
    FROM mining.mv_hash_rate_stats
    WHERE channel_id = 1 AND period_start BETWEEN now() - ()
)
FORMAT Pretty"

## то же самое из shares

curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT 
    total_shares,
    total_hashes,
    total_hashes / 3600 AS hashrate
FROM (
    SELECT 
        count() AS total_shares,
        sum(difficulty * pow(2, 32)) AS total_hashes
    FROM mining.shares
    WHERE channel_id = 1 AND timestamp BETWEEN now() - INTERVAL 1 HOUR AND now()
)
FORMAT Pretty"

####

# 1. Проверка количества записей в таблице shares
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT count() as total_shares FROM mining.shares FORMAT Pretty"

# 2. Проверка распределения по channel_id
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT 
    channel_id,
    count() as shares_count
FROM mining.shares 
GROUP BY channel_id
ORDER BY channel_id
FORMAT Pretty"

# 4. Проверка последних записей (чтобы увидеть структуру данных)
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT * FROM mining.shares
ORDER BY timestamp DESC
LIMIT 5
FORMAT Pretty"

# 5. Распределение share_status
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT 
    share_status,
    count() as count
FROM mining.shares
GROUP BY share_status
ORDER BY share_status
FORMAT Pretty"

