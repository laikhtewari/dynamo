#!/usr/bin/env python3
# SPDX-FileCopyrightText: Copyright (c) 2025 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

"""
Example demonstrating the new prom_metric API for declarative metrics registration.

This shows how Python code can:
1. Create metric objects using prom_metric()
2. Register them with an endpoint (supports Gauge, IntGauge, GaugeVec, IntGaugeVec)
3. Update their values (which updates the underlying Prometheus gauges)
4. The metrics are automatically served via the /metrics endpoint
"""

import asyncio

import uvloop

from dynamo._core import prom_metric
from dynamo.runtime import DistributedRuntime, dynamo_worker


@dynamo_worker()
async def worker(runtime: DistributedRuntime) -> None:
    await init(runtime)


async def init(runtime: DistributedRuntime):
    # Create component and endpoint
    component = runtime.namespace("ns556").component("cp556")
    await component.create_service()

    generate_endpoint = component.endpoint("ep556")

    # Step 1: Declare metrics using prom_metric()
    print("[python] Creating metrics...")

    # Simple metrics (Gauge and IntGauge)
    request_total_slots = prom_metric("request_total_slots", "int", 2048)
    gpu_cache_usage_perc = prom_metric("gpu_cache_usage_percent", "float", 0.0)

    # Vector metrics (GaugeVec with labels)
    worker_active_requests = prom_metric(
        "worker_active_requests", "int", label_names=["worker_id", "model"]
    )

    print(f"[python] Created: {request_total_slots}")
    print(f"[python] Created: {gpu_cache_usage_perc}")
    print(f"[python] Created: {worker_active_requests}")

    # Step 2: Register these metrics with the endpoint
    print("[python] Registering metrics with endpoint...")
    list_of_metrics = [
        request_total_slots,
        gpu_cache_usage_perc,
        worker_active_requests,
    ]
    generate_endpoint.register_metrics(list_of_metrics)
    print("[python] Metrics registered!")

    # Step 3: Register a callback to update metrics dynamically
    print("[python] Registering metrics callback...")

    # Simulated counter for demonstration
    counter = {"value": 0}

    def update_metrics():
        """Called automatically before /metrics endpoint is scraped"""
        counter["value"] += 1
        # Update metrics with fresh values
        request_total_slots.set(1024 + counter["value"])
        gpu_cache_usage_perc.set(0.75 + (counter["value"] * 0.01))
        print(f"[python] Updated metrics (call #{counter['value']})")

    generate_endpoint.register_metrics_callback(update_metrics)
    print("[python] Metrics callback registered!")

    # Step 4: Set initial values and test vector metrics
    print("[python] Setting initial metric values...")
    request_total_slots.set(1024)
    gpu_cache_usage_perc.set(0.75)
    print(f"[python] request_total_slots = {request_total_slots.get()}")
    print(f"[python] gpu_cache_usage_perc = {gpu_cache_usage_perc.get()}")

    print("[python] Updating vector metric with labels...")
    worker_active_requests.set(5, {"worker_id": "worker_1", "model": "llama-3"})
    worker_active_requests.set(3, {"worker_id": "worker_2", "model": "llama-3"})
    print("[python] worker_active_requests set for worker_1 and worker_2")

    # The metrics are now available at:
    # http://localhost:<system_status_port>/metrics
    print("[python] ✅ Metrics are now registered and served via /metrics endpoint")
    print(
        "[python]    Check the system status server port to see them in Prometheus format"
    )
    print("[python]    Supported types: Gauge, IntGauge, GaugeVec, IntGaugeVec")

    # Note: This example does not call serve_endpoint() to keep it simple.
    # In a real service, you would call: await generate_endpoint.serve_endpoint(handler, ...)
    # Keep running so metrics endpoint stays up
    _ = await asyncio.Event().wait()


if __name__ == "__main__":
    uvloop.install()
    asyncio.run(worker())
