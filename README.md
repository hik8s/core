# HiK8s Core

This repository contains **data processing** and **backend** of the HiK8s Kubernetes chatbot.

**Note** This code is not under active development anymore. We recommend to use tools like [kagent](https://github.com/kagent-dev/kagent) or [k8sgpt](https://github.com/k8sgpt-ai/k8sgpt).

## Introduction

The code is separated into the following modules:

| Component | Main File | Cargo.toml | Documentation |
|-----------|-----------|------------|---------------|
| Data Intake       | [main.rs](./rs/data-intake/src/main.rs)       | [Cargo.toml](./rs/data-intake/Cargo.toml)     | [README.md](./rs/data-intake/README.md)       |
| Data Processing   | [main.rs](./rs/data-processing/src/main.rs)   | [Cargo.toml](./rs/data-processing/Cargo.toml) | [README.md](./rs/data-processing/README.md)   |
| Data Vectorizer   | [main.rs](./rs/data-vectorizer/src/main.rs)   | [Cargo.toml](./rs/data-vectorizer/Cargo.toml) | [README.md](./rs/data-vectorizer/README.md)   |
| Chat Backend      | [main.rs](./rs/chat-backend/src/main.rs)      | [Cargo.toml](./rs/chat-backend/Cargo.toml)    | [README.md](./rs/chat-backend/README.md)      |
| Shared Library    | [lib.rs](./rs/shared/src/lib.rs)              | [Cargo.toml](./rs/shared/Cargo.toml)          | [README.md](./rs/shared/README.md)            |
| End-to-End Tests  | [lib.rs](./rs/tests/src/lib.rs)               | [Cargo.toml](./rs/tests/Cargo.toml)           | [README.md](./rs/tests/README.md)             |

## Setup

The following are manual commands to setup necessary databases for HiK8s.

### GreptimeDB (Time-Series DB)

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

### Qdrant (Vector DB)

```bash
docker pull qdrant/qdrant:v1.11.3
docker run -d -p 6333:6333 -p 6334:6334 \
    -v $(pwd)/qdrant_storage:/qdrant/storage:z \
    qdrant/qdrant:v1.11.3
```

### Redis (In-Memory Cache)

```bash
docker run -d --name redis -p 6379:6379 redis redis-server --requirepass $REDIS_PASSWORD
```

### Fluvio (message broker)

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

## Data Intake

Test command for data-intake.

```bash
# FILE=/tmp/records.txt
# URL="https://dev.api.hik8s.ai"
# URL="https://stag.api.hik8s.ai"
FILE="/tmp/records.txt"
# URL="https://dev.api.hik8s.ai"
URL="http://localhost:8000"
curl -X POST \
     -H "Authorization: Bearer $AUTH_TOKEN" \
     -F 'metadata={"path": "/var/log/pods/ns_my-pod-'$RANDOM'_uid-123/container", "file": "file_name_value"};type=application/json' \
     -F "stream=@$FILE;type=application/octet-stream" \
     $URL/logs
```
