<!-- SPDX-FileCopyrightText: Copyright (c) 2025 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Python-Rust Metrics Integration

This directory demonstrates two methods for passing metrics between Python and Rust in the Dynamo runtime.

## Method 1: ForwardPassMetrics Pub/Sub via NATS with serde (Legacy method for passing metrics)

Python maintains its own metrics dictionary, serializes it, and publishes to NATS. Rust subscribes to NATS, deserializes the metrics, and updates Prometheus gauges.

**Example**: Used by `WorkerMetricsPublisher` in production code

```python
from dynamo.llm import WorkerMetricsPublisher, ForwardPassMetrics

# Create publisher
publisher = WorkerMetricsPublisher()
await publisher.create_endpoint(component, metrics_labels)

# Python maintains its own metrics dict
metrics_dict = {
    "num_running_reqs": 5,
    "num_waiting_reqs": 10,
    "gpu_cache_usage": 0.75,
}

# Serialize and publish to NATS
metrics = ForwardPassMetrics(metrics_dict)
publisher.publish(metrics)

# Rust subscribes to NATS, deserializes, and updates Prometheus
```

### Adding/Changing Metrics in Method 1

When you need to add or modify metrics in Method 1 (ForwardPassMetrics Pub/Sub via NATS with serde), you must update **multiple files**:

1. **`lib/llm/src/kv_router/protocols.rs`** - Add field to struct:
   ```rust
   pub struct WorkerStats {
       pub request_active_slots: u64,
       pub request_total_slots: u64,
       pub num_requests_waiting: u64,
       pub new_metric_field: u64,  // ADD THIS
   }
   ```

2. **`lib/llm/src/kv_router/publisher.rs`** - Manually create Prometheus gauge using DRT:
   ```rust
   fn new(component: &Component) -> Result<Self> {
       use dynamo_runtime::metrics::MetricsRegistry;

       // ... existing gauges ...

       // Manually create and register new Prometheus gauge
       let new_metric_gauge = component.create_gauge(
           "new_metric_name",
           "Description of new metric",
           &[],  // labels
       )?;

       // Store in struct
       Ok(KvStatsPrometheusGauges {
           kv_active_blocks_gauge,
           kv_total_blocks_gauge,
           gpu_cache_usage_gauge,
           gpu_prefix_cache_hit_rate_gauge,
           new_metric_gauge,  // ADD THIS
       })
   }
   ```

3. **`lib/llm/src/kv_router/publisher.rs`** - Update gauge in `update_from_kvstats()`:
   ```rust
   fn update_from_kvstats(&self, kv_stats: &KvStats) {
       // ... existing updates ...
       self.new_metric_gauge.set(worker_stats.new_metric_field as f64);
   }
   ```

4. **`components/backends/sglang/.../publisher.py`** - Update Python code to compute new metric:
   ```python
   def collect_metrics():
       worker_stats = WorkerStats(
           request_active_slots=...,
           new_metric_field=compute_new_metric(),  # ADD THIS
       )
   ```

**Result**: Changes require touching 3-4 files across Rust and Python codebases.

## Method 2: Declarative/Registration-based (New method for passing metrics)

Python creates metric objects using `prom_metric()`, registers them with an endpoint, and updates values through these objects. Rust creates the underlying Prometheus gauges and calls Python callbacks before scraping.

**Example**: `server.py`

```python
# Create metric objects
request_slots = prom_metric("request_total_slots", "int", 2048)
gpu_usage = prom_metric("gpu_cache_usage_percent", "float", 0.0)

# Register with endpoint
endpoint.register_metrics([request_slots, gpu_usage])

# Register callback for dynamic updates
def update_metrics():
    request_slots.set(compute_slots())
    gpu_usage.set(compute_gpu_usage())

endpoint.register_metrics_callback(update_metrics)
```

### Adding/Changing Metrics in Method 2

When you need to add or modify metrics in Method 2 (Declarative), you only update **Python code**:

1. **Create new metric** - Just add one line in Python:
   ```python
   new_metric = prom_metric("new_metric_name", "int", 0)
   ```

2. **Register it** - Add to the list:
   ```python
   endpoint.register_metrics([request_slots, gpu_usage, new_metric])
   ```

3. **Update in callback** - Add update logic:
   ```python
   def update_metrics():
       request_slots.set(compute_slots())
       gpu_usage.set(compute_gpu_usage())
       new_metric.set(compute_new_metric())  # ADD THIS
   ```

**Result**: Changes only require modifying Python code. No Rust changes needed. The Prometheus gauge is automatically created and registered by the Rust runtime.

## Architecture Diagrams

### Component Architecture

#### Method 1: ForwardPassMetrics Pub/Sub via NATS with serde - Component View

```mermaid
graph TB
    subgraph "Python Layer"
        PY[Python Application<br/>components/backends/sglang/main.py]
        style PY fill:#3776ab,color:#fff
    end

    subgraph "Python/Rust Interface (PyO3)"
        WMPB[WorkerMetricsPublisher Bindings<br/>bindings/python/rust/llm/kv.rs]
        FPM[ForwardPassMetrics Struct<br/>bindings/python/rust/llm/kv.rs]
        style WMPB fill:#f4a261,color:#000
        style FPM fill:#f4a261,color:#000
    end

    subgraph "Rust Core"
        subgraph "Worker Process Components"
            WMP[WorkerMetricsPublisher<br/>llm/src/kv_router/publisher.rs]
            WATCH[Watch Channel<br/>tokio::sync::watch]
            PROM1[Local Prometheus Gauges<br/>prometheus::Gauge]
        end

        subgraph "NATS Infrastructure"
            NATS[NATS Server<br/>KV_METRICS_SUBJECT]
        end

        subgraph "Aggregator Process Components"
            AGG[KvMetricsAggregator<br/>llm/src/kv_router/metrics_aggregator.rs]
            SUB[NATS Subscriber<br/>component/namespace.rs]
        end

        subgraph "System Status Servers"
            SS[System Status Server<br/>runtime/src/system_status_server.rs<br/>Started by DistributedRuntime]
        end

        style WMP fill:#ce422b,color:#fff
        style WATCH fill:#ce422b,color:#fff
        style PROM1 fill:#ce422b,color:#fff
        style NATS fill:#27aae1,color:#fff
        style AGG fill:#ce422b,color:#fff
        style SUB fill:#ce422b,color:#fff
        style SS fill:#6c757d,color:#fff
    end

    PY -->|"WorkerMetricsPublisher()"| WMPB
    PY -->|"ForwardPassMetrics(worker_stats, kv_stats, spec_decode_stats)"| FPM
    PY -->|"publish(metrics)"| WMPB
    WMPB -->|"FFI: publish(Arc ForwardPassMetrics)"| WMP
    WMP -->|"update_from_kvstats(kv_stats)"| PROM1
    WMP -->|"tx.send(metrics)"| WATCH
    WATCH -->|"publish(KV_METRICS_SUBJECT, LoadEvent)"| NATS
    NATS -->|"subscribe_with_type LoadEvent"| SUB
    SUB -->|"discover endpoints"| AGG
    SS -->|"Worker: gather() from PROM1"| PROM1
    SS -->|"Aggregator: scrape_stats()"| AGG
```

#### Method 2: Declarative/Registration-based - Component View

```mermaid
graph TB
    subgraph "Python Layer"
        PY[Python Application<br/>main.py]
        style PY fill:#3776ab,color:#fff
    end

    subgraph "Python/Rust Interface (PyO3)"
        DPM[DynamoPromMetric<br/>bindings/python/rust/metrics.rs]
        EP[Endpoint Bindings<br/>bindings/python/rust/lib.rs]
        style DPM fill:#f4a261,color:#000
        style EP fill:#f4a261,color:#000
    end

    subgraph "Rust Core"
        MR[MetricsRegistry Trait<br/>runtime/src/metrics.rs]
        DRT[DistributedRuntime<br/>runtime/src/distributed.rs]
        PROM[Prometheus Crate<br/>prometheus::Gauge/IntGauge]
        SS[System Status Server<br/>runtime/src/system_status_server.rs]
        style MR fill:#ce422b,color:#fff
        style DRT fill:#ce422b,color:#fff
        style PROM fill:#ce422b,color:#fff
        style SS fill:#ce422b,color:#fff
    end

    PY -->|"prom_metric(name, type, value)"| DPM
    PY -->|"register_metrics(List[DynamoPromMetric])"| EP
    PY -->|"register_metrics_callback(callback)"| EP
    DPM -.->|"set(value) / get() -> value"| PROM
    EP -->|"create_intgauge(name, help, labels)"| MR
    EP -->|"register_metrics_callback(hierarchies, callback)"| DRT
    MR -->|"register_metric(metric)"| PROM
    DRT -->|"execute_metrics_callbacks(hierarchy)"| EP
    EP -.->|"callback() -> None"| PY
    SS -->|"execute_metrics_callbacks(all hierarchies)"| DRT
    SS -->|"prometheus_metrics_fmt() -> String"| PROM
```

### Sequence Diagrams

#### Method 1: ForwardPassMetrics Pub/Sub via NATS with serde

```mermaid
sequenceDiagram
    autonumber
    participant P as Python Worker<br/>(components/backends/sglang)
    participant WMP as WorkerMetricsPublisher<br/>(bindings/python/rust/llm/kv.rs)
    participant N as NATS
    participant A as Aggregator<br/>(llm/src/kv_router/metrics_aggregator.rs)
    participant PR as Prometheus Registry<br/>(runtime/src/metrics.rs)
    participant S as /metrics Scraper<br/>(runtime/src/system_status_server.rs)

    Note over P,WMP: Setup Phase
    P->>WMP: WorkerMetricsPublisher()<br/>kv.rs:WorkerMetricsPublisher::new()
    WMP->>WMP: create watch channel<br/>publisher.rs:WorkerMetricsPublisher::new()
    P->>WMP: register_prometheus_metrics(component)<br/>kv.rs:register_prometheus_metrics()
    WMP->>PR: register KvStats gauges<br/>publisher.rs:KvStatsPrometheusGauges::new()
    WMP->>N: subscribe to KV_METRICS_SUBJECT<br/>publisher.rs:start_nats_metrics_publishing()

    Note over P,WMP: Runtime Phase (continuous)
    P->>P: compute WorkerStats, KvStats, SpecDecodeStats
    P->>P: ForwardPassMetrics(worker_stats, kv_stats, spec_decode_stats)<br/>kv.rs:ForwardPassMetrics::new()
    P->>WMP: publish(metrics)<br/>kv.rs:WorkerMetricsPublisher::publish()
    WMP->>PR: update local gauges immediately<br/>publisher.rs:update_from_kvstats()
    WMP->>WMP: watch channel (debounce 1ms)<br/>publisher.rs:start_nats_metrics_publishing()
    WMP->>WMP: serde_json::to_vec(LoadEvent)<br/>component/namespace.rs:EventPublisher::publish()
    WMP->>N: NATS publish(json_bytes)<br/>component/namespace.rs:publish_bytes()

    Note over N,A: Aggregation Phase
    N->>A: receive LoadEvent message<br/>component/namespace.rs:subscribe_with_type()
    A->>A: serde_json::from_slice()<br/>component/namespace.rs:subscribe_with_type()
    A->>A: aggregate metrics from workers<br/>metrics_aggregator.rs:KvMetricsAggregator
    A->>PR: update aggregated gauges<br/>metrics_aggregator.rs:update_gauges()

    Note over S,PR: Scrape Phase (worker or aggregator)
    S->>A: GET /metrics (aggregator)<br/>system_status_server.rs:metrics_handler()
    A->>PR: gather() aggregated<br/>metrics.rs:prometheus_metrics_fmt()
    PR-->>S: Prometheus text format
    S->>WMP: GET /metrics (worker)<br/>system_status_server.rs:metrics_handler()
    WMP->>PR: gather() local<br/>metrics.rs:prometheus_metrics_fmt()
    PR-->>S: Prometheus text format
```

#### Method 2: Declarative/Registration-based

```mermaid
sequenceDiagram
    autonumber
    participant P as Python
    participant DPM as DynamoPromMetric<br/>(bindings/python/rust/metrics.rs)
    participant R as Rust Endpoint<br/>(bindings/python/rust/lib.rs)
    participant PR as Prometheus Registry<br/>(runtime/src/metrics.rs)
    participant S as /metrics Scraper<br/>(runtime/src/system_status_server.rs)

    Note over P,DPM: Setup Phase
    P->>DPM: prom_metric("name", "int", 0)<br/>metrics.rs:prom_metric()
    DPM-->>P: metric object
    P->>R: register_metrics([metric])<br/>lib.rs:Endpoint::register_metrics()
    R->>PR: create_intgauge("name")<br/>metrics.rs:MetricsRegistry::create_intgauge()
    PR-->>R: Prometheus IntGauge
    R->>DPM: set_prometheus_intgauge(gauge)<br/>metrics.rs:DynamoPromMetric::set_prometheus_intgauge()

    P->>R: register_metrics_callback(update_fn)<br/>lib.rs:Endpoint::register_metrics_callback()
    R-->>R: store callback<br/>distributed.rs:register_metrics_callback()

    Note over P,DPM: Update Phase
    P->>DPM: metric.set(1024)<br/>metrics.rs:DynamoPromMetric::set()
    DPM->>PR: gauge.set(1024)

    Note over S,PR: Scrape Phase
    S->>R: GET /metrics<br/>system_status_server.rs:metrics_handler()
    R->>P: call update_fn()<br/>distributed.rs:execute_metrics_callbacks()
    P->>DPM: metric.set(new_value)
    DPM->>PR: gauge.set(new_value)
    R->>PR: gather()<br/>metrics.rs:prometheus_metrics_fmt()
    PR-->>S: Prometheus text format
```

## Comparison

| Aspect | Method 1: ForwardPassMetrics Pub/Sub | Method 2: Declarative |
|--------|----------------------|------------------------|
| **Ownership** | Python owns metrics dict, Rust owns Prometheus objects | Python holds metric objects, Rust holds Prometheus objects |
| **Communication** | Indirect via NATS message broker | Direct Foreign Function Interface (callbacks from Rust to Python) |
| **Update Pattern** | Serialize entire dict and publish | `.set()` on individual metric objects |
| **Serialization** | JSON/MessagePack serialization to NATS | No serialization (direct FFI calls) |
| **Overhead** | Medium (NATS network + serialization) | Higher (Python-Rust FFI per metric update) |
| **Decoupling** | Loosely coupled (can run in different processes) | Tightly coupled (Python and Rust in same process) |
| **Scalability** | Multiple workers publish to same topic | Single worker only |
| **Flexibility** | Push-based, may have stale values | Callback ensures fresh values before scrape |
| **Complexity** | High (NATS setup, struct changes, Rust+Python) | Low (Python-only, simple API) |
| **Use Case** | Distributed workers publishing to aggregator | Single-process services with dynamic updates |
