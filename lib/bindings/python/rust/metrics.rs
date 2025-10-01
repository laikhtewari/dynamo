// SPDX-FileCopyrightText: Copyright (c) 2025 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Internal enum to hold different types of Prometheus gauges
#[derive(Clone)]
pub(crate) enum PrometheusGauge {
    Gauge(prometheus::Gauge),
    IntGauge(prometheus::IntGauge),
    GaugeVec(prometheus::GaugeVec),
    IntGaugeVec(prometheus::IntGaugeVec),
}

impl PrometheusGauge {
    fn set(&self, value: f64, labels: Option<&HashMap<String, String>>) -> Result<(), String> {
        match self {
            PrometheusGauge::Gauge(g) => {
                g.set(value);
                Ok(())
            }
            PrometheusGauge::IntGauge(g) => {
                g.set(value as i64);
                Ok(())
            }
            PrometheusGauge::GaugeVec(gv) => {
                if let Some(labels) = labels {
                    let label_values: Vec<&str> = labels.values().map(|s| s.as_str()).collect();
                    gv.with_label_values(&label_values).set(value);
                    Ok(())
                } else {
                    Err("GaugeVec requires labels".to_string())
                }
            }
            PrometheusGauge::IntGaugeVec(gv) => {
                if let Some(labels) = labels {
                    let label_values: Vec<&str> = labels.values().map(|s| s.as_str()).collect();
                    gv.with_label_values(&label_values).set(value as i64);
                    Ok(())
                } else {
                    Err("IntGaugeVec requires labels".to_string())
                }
            }
        }
    }

    fn get(&self, labels: Option<&HashMap<String, String>>) -> Result<f64, String> {
        match self {
            PrometheusGauge::Gauge(g) => Ok(g.get()),
            PrometheusGauge::IntGauge(g) => Ok(g.get() as f64),
            PrometheusGauge::GaugeVec(gv) => {
                if let Some(labels) = labels {
                    let label_values: Vec<&str> = labels.values().map(|s| s.as_str()).collect();
                    Ok(gv.with_label_values(&label_values).get())
                } else {
                    Err("GaugeVec requires labels".to_string())
                }
            }
            PrometheusGauge::IntGaugeVec(gv) => {
                if let Some(labels) = labels {
                    let label_values: Vec<&str> = labels.values().map(|s| s.as_str()).collect();
                    Ok(gv.with_label_values(&label_values).get() as f64)
                } else {
                    Err("IntGaugeVec requires labels".to_string())
                }
            }
        }
    }
}

/// Python wrapper around Prometheus gauges in Rust
/// Supports Gauge, IntGauge, GaugeVec, and IntGaugeVec
#[pyclass]
pub struct DynamoPromMetric {
    name: String,
    metric_type: String, // "int" or "float"
    is_vec: bool,        // Whether this is a GaugeVec
    label_names: Option<Vec<String>>, // Label names for GaugeVec
    labels: Option<HashMap<String, String>>, // Label values for GaugeVec operations
    gauge: Arc<Mutex<Option<PrometheusGauge>>>,
}

#[pymethods]
impl DynamoPromMetric {
    /// Create a new DynamoPromMetric (does not register with Prometheus yet)
    #[new]
    #[pyo3(signature = (name, metric_type, initial_value=None, label_names=None, labels=None))]
    fn new(
        name: String,
        metric_type: String,
        initial_value: Option<f64>,
        label_names: Option<Vec<String>>,
        labels: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        // Validate metric_type
        if metric_type != "int" && metric_type != "float" {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "metric_type must be 'int' or 'float'",
            ));
        }

        let is_vec = label_names.is_some();

        // If label_names provided, validate labels
        if is_vec {
            if let Some(ref labels) = labels {
                let label_names_ref = label_names.as_ref().unwrap();
                if labels.len() != label_names_ref.len() {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Number of labels must match number of label_names",
                    ));
                }
            }
        }

        let metric = Self {
            name,
            metric_type,
            is_vec,
            label_names,
            labels: labels.clone(),
            gauge: Arc::new(Mutex::new(None)),
        };

        // Set initial value if provided
        if let Some(value) = initial_value {
            metric.set(value, labels)?;
        }

        Ok(metric)
    }

    /// Get the metric name
    #[getter]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get the metric type
    #[getter]
    pub fn metric_type(&self) -> String {
        self.metric_type.clone()
    }

    /// Check if this is a vector metric
    #[getter]
    pub fn is_vec(&self) -> bool {
        self.is_vec
    }

    /// Get label names (for GaugeVec)
    #[getter]
    pub fn label_names(&self) -> Option<Vec<String>> {
        self.label_names.clone()
    }

    /// Set the metric value
    #[pyo3(signature = (value, labels=None))]
    fn set(&self, value: f64, labels: Option<HashMap<String, String>>) -> PyResult<()> {
        let gauge_opt = self.gauge.lock().unwrap();
        if let Some(ref gauge) = *gauge_opt {
            let labels_to_use = labels.as_ref().or(self.labels.as_ref());
            gauge.set(value, labels_to_use).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e)
            })?;
        } else {
            tracing::trace!(
                "Metric '{}' not yet registered with Prometheus",
                self.name
            );
        }
        Ok(())
    }

    /// Get the current metric value
    #[pyo3(signature = (labels=None))]
    fn get(&self, labels: Option<HashMap<String, String>>) -> PyResult<f64> {
        let gauge_opt = self.gauge.lock().unwrap();
        if let Some(ref gauge) = *gauge_opt {
            let labels_to_use = labels.as_ref().or(self.labels.as_ref());
            gauge.get(labels_to_use).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e)
            })
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Metric not yet registered with Prometheus",
            ))
        }
    }

    /// Set labels for this metric (for GaugeVec operations)
    fn with_labels(&self, labels: HashMap<String, String>) -> PyResult<Self> {
        if !self.is_vec {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot set labels on non-vector metric",
            ));
        }

        Ok(Self {
            name: self.name.clone(),
            metric_type: self.metric_type.clone(),
            is_vec: self.is_vec,
            label_names: self.label_names.clone(),
            labels: Some(labels),
            gauge: self.gauge.clone(),
        })
    }

    fn __repr__(&self) -> String {
        if self.is_vec {
            format!(
                "DynamoPromMetric('{}', type={}, vec=true, label_names={:?})",
                self.name, self.metric_type, self.label_names
            )
        } else {
            format!("DynamoPromMetric('{}', type={})", self.name, self.metric_type)
        }
    }
}

impl DynamoPromMetric {
    /// Internal method to set the actual Prometheus gauge (called during registration)
    pub(crate) fn set_gauge(&self, gauge: PrometheusGauge) {
        let mut gauge_opt = self.gauge.lock().unwrap();
        *gauge_opt = Some(gauge);
    }

    /// Internal method to set a regular Gauge
    pub fn set_prometheus_gauge(&self, gauge: prometheus::Gauge) {
        self.set_gauge(PrometheusGauge::Gauge(gauge));
    }

    /// Internal method to set an IntGauge
    pub fn set_prometheus_intgauge(&self, gauge: prometheus::IntGauge) {
        self.set_gauge(PrometheusGauge::IntGauge(gauge));
    }

    /// Internal method to set a GaugeVec
    pub fn set_prometheus_gaugevec(&self, gauge: prometheus::GaugeVec) {
        self.set_gauge(PrometheusGauge::GaugeVec(gauge));
    }

    /// Internal method to set an IntGaugeVec
    pub fn set_prometheus_intgaugevec(&self, gauge: prometheus::IntGaugeVec) {
        self.set_gauge(PrometheusGauge::IntGaugeVec(gauge));
    }
}

/// Helper function to create a DynamoPromMetric - this is the main API function
#[pyfunction]
#[pyo3(signature = (name, metric_type, initial_value=None, label_names=None, labels=None))]
pub fn prom_metric(
    name: String,
    metric_type: String,
    initial_value: Option<f64>,
    label_names: Option<Vec<String>>,
    labels: Option<HashMap<String, String>>,
) -> PyResult<DynamoPromMetric> {
    DynamoPromMetric::new(name, metric_type, initial_value, label_names, labels)
}

/// Add metrics bindings to the Python module
pub fn add_to_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DynamoPromMetric>()?;
    m.add_function(wrap_pyfunction!(prom_metric, m)?)?;
    Ok(())
}
