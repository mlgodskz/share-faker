cd /home/ro/projects/nomium/01_SRI-Local-Utils/share-faker
cargo run

#### ClickHouse посчитать записи
curl "http://localhost:8123/?query=SELECT%20COUNT(*)%20FROM%20mining.fake_share_logs" \
  -H "X-ClickHouse-User: default" \
  -H "X-ClickHouse-Key: 5555"
## посмотреть записи 5 штук
curl "http://localhost:8123/?query=SELECT%20*%20FROM%20mining.fake_share_logs%20LIMIT%205%20FORMAT%20JSON" \
  -H "X-ClickHouse-User: default" \
  -H "X-ClickHouse-Key: 5555"
## 10 штук
curl "http://localhost:8123/?query=SELECT%20*%20FROM%20mining.fake_share_logs%20LIMIT%2010%20FORMAT%20JSON" \
  -H "X-ClickHouse-User: default" \
  -H "X-ClickHouse-Key: 5555"
## Удалить таблицу
curl "http://localhost:8123/" \
  -H "X-ClickHouse-User: default" \
  -H "X-ClickHouse-Key: 5555" \
  -d "DROP TABLE mining.fake_share_logs"

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

# 3. Хэшрейт за последний час по каждому воркеру
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT 
    channel_id,
    sum(total_hashes) / 3600 as hashrate_per_second,
    sum(share_count) as shares_count
FROM mining.mv_hash_rate_stats 
WHERE period_start >= now() - INTERVAL 1 HOUR
GROUP BY channel_id
ORDER BY hashrate_per_second DESC
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

# 6. Детальная почасовая статистика для конкретного воркера
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT 
    channel_id,
    toStartOfHour(period_start) as hour,
    sum(share_count) as shares,
    sum(total_hashes) / 3600 as hashrate_per_second
FROM mining.mv_hash_rate_stats 
WHERE channel_id = 1
  AND period_start >= now() - INTERVAL 24 HOUR
GROUP BY channel_id, hour
ORDER BY hour DESC
FORMAT Pretty"
