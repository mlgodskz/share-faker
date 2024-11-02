cd /home/ro/projects/Stratum-RedRock_Pool/01_SRI-Local-Utils/share-faker
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