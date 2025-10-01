# SPDX-FileCopyrightText: Copyright (c) 2025 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

from typing import Dict, List, Optional

class DynamoPromMetric:
    """
    Python wrapper around Prometheus gauges in Rust.

    Supports Gauge, IntGauge, GaugeVec, and IntGaugeVec types.
    This allows Python code to declare metrics and later register them with DRT.
    Once registered, the metrics are automatically served via the /metrics endpoint.
    """

    def __init__(
        self,
        name: str,
        metric_type: str,
        initial_value: Optional[float] = None,
        label_names: Optional[List[str]] = None,
        labels: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Create a new DynamoPromMetric.

        Args:
            name: The metric name
            metric_type: Either "int" or "float"
            initial_value: Optional initial value for the metric
            label_names: Optional list of label names (for GaugeVec/IntGaugeVec)
            labels: Optional dict of label values (for setting vector metrics)
        """
        ...

    @property
    def name(self) -> str:
        """Get the metric name"""
        ...

    @property
    def metric_type(self) -> str:
        """Get the metric type ('int' or 'float')"""
        ...

    @property
    def is_vec(self) -> bool:
        """Check if this is a vector metric (GaugeVec/IntGaugeVec)"""
        ...

    @property
    def label_names(self) -> Optional[List[str]]:
        """Get the label names for vector metrics"""
        ...

    def set(self, value: float, labels: Optional[Dict[str, str]] = None) -> None:
        """
        Set the metric value.

        If the metric has been registered, this updates the underlying Prometheus gauge.
        For vector metrics, labels must be provided either here or during initialization.

        Args:
            value: The new metric value
            labels: Optional dict of label values (required for vector metrics)
        """
        ...

    def get(self, labels: Optional[Dict[str, str]] = None) -> float:
        """
        Get the current metric value.

        Args:
            labels: Optional dict of label values (required for vector metrics)

        Returns:
            The current value of the Prometheus gauge

        Raises:
            RuntimeError: If the metric has not yet been registered
        """
        ...

    def with_labels(self, labels: Dict[str, str]) -> DynamoPromMetric:
        """
        Create a new DynamoPromMetric instance with bound labels for vector metrics.

        Args:
            labels: Dict of label values to bind

        Returns:
            A new DynamoPromMetric instance with labels set

        Raises:
            ValueError: If this is not a vector metric
        """
        ...

def prom_metric(
    name: str,
    metric_type: str,
    initial_value: Optional[float] = None,
    label_names: Optional[List[str]] = None,
    labels: Optional[Dict[str, str]] = None,
) -> DynamoPromMetric:
    """
    Create a DynamoPromMetric wrapper around Prometheus gauges.

    This is the main API for creating declarative metrics in Python.
    Supports Gauge, IntGauge, GaugeVec, and IntGaugeVec.

    Example:
        ```python
        # Simple metrics (Gauge/IntGauge)
        request_total_slots = prom_metric("request_total_slots", "int", 2048)
        gpu_cache_usage = prom_metric("gpu_cache_usage_percent", "float", 0.0)

        # Vector metrics (GaugeVec/IntGaugeVec)
        worker_requests = prom_metric(
            "worker_active_requests",
            "int",
            label_names=["worker_id", "model"]
        )

        # Register with endpoint
        endpoint.register_metrics([request_total_slots, gpu_cache_usage, worker_requests])

        # Update simple metrics
        request_total_slots.set(1024)
        gpu_cache_usage.set(0.75)

        # Update vector metrics with labels
        worker_requests.set(5, {"worker_id": "worker_1", "model": "llama-3"})
        worker_requests.set(3, {"worker_id": "worker_2", "model": "llama-3"})
        ```

    Args:
        name: The metric name (should match Prometheus naming conventions)
        metric_type: Either "int" (IntGauge/IntGaugeVec) or "float" (Gauge/GaugeVec)
        initial_value: Optional initial value for the metric
        label_names: Optional list of label names (creates GaugeVec/IntGaugeVec if provided)
        labels: Optional dict of label values (for setting vector metrics)

    Returns:
        A DynamoPromMetric object that can be registered with an endpoint
    """
    ...
