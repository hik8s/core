# GreptimeDB (timeseries database)

```bash
brew install protobuf
```

```bash
docker pull greptime/greptimedb:v0.9.1
docker run -d -p 4000-4003:4000-4003 --name greptime \
greptime/greptimedb:v0.9.1 standalone start \
--http-addr 0.0.0.0:4000 \
--rpc-addr 0.0.0.0:4001 \
--mysql-addr 0.0.0.0:4002 \
--postgres-addr 0.0.0.0:4003
```

## Redis

```bash
docker run -d --name redis -p 6379:6379 redis redis-server --requirepass $REDIS_PASSWORD
```

## Fluvio

Install fluvio:

```bash
curl -fsS https://hub.infinyon.cloud/install/install.sh | bash
```

Start cluster:

```bash
fluvio cluster start
```

Resume cluster:

```bash
fluvio cluster resume
```

## Qdrant

```bash
docker pull qdrant/qdrant:v1.9.7
docker run -d -p 6333:6333 -p 6334:6334 \
    -v $(pwd)/qdrant_storage:/qdrant/storage:z \
    qdrant/qdrant:v1.9.7
```
