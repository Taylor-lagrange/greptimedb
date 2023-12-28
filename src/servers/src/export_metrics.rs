// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::time::Duration;

use axum::http::HeaderValue;
use common_base::Plugins;
use common_telemetry::metric::{convert_metric_to_write_request, MetricFilter};
use common_telemetry::{error, info};
use common_time::Timestamp;
use hyper::HeaderMap;
use prost::Message;
use reqwest::header::HeaderName;
use serde::{Deserialize, Serialize};
use session::context::QueryContextBuilder;
use snafu::{ensure, ResultExt};
use tokio::time::{self, Interval};

use crate::error::{InvalidExportMetricsConfigSnafu, Result, SendPromRemoteRequestSnafu};
use crate::prom_store::snappy_compress;
use crate::query_handler::PromStoreProtocolHandlerRef;

/// Use to export the metrics generated by greptimedb, encoded to Prometheus [RemoteWrite format](https://prometheus.io/docs/concepts/remote_write_spec/),
/// and send to Prometheus remote-write compatible receiver (e.g. send to `greptimedb` itself)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ExportMetricsOption {
    pub enable: bool,
    pub endpoint: String,
    pub db: String,
    #[serde(with = "humantime_serde")]
    pub write_interval: Duration,
    pub headers: HashMap<String, String>,
}

impl Default for ExportMetricsOption {
    fn default() -> Self {
        Self {
            enable: false,
            endpoint: "127.0.0.1:4000".to_string(),
            db: "information_schema".to_string(),
            write_interval: Duration::from_secs(30),
            headers: HashMap::new(),
        }
    }
}

#[derive(Default, Clone)]
pub struct ExportMetricsTask {
    config: ExportMetricsOption,
    filter: Option<MetricFilter>,
    headers: HeaderMap<HeaderValue>,
    pub send_by_handler: bool,
    handler: Option<PromStoreProtocolHandlerRef>,
}

impl ExportMetricsTask {
    pub fn try_new(
        config: &ExportMetricsOption,
        http_addr: Option<&str>,
        plugins: Option<&Plugins>,
    ) -> Result<Option<Self>> {
        if !config.enable {
            return Ok(None);
        }
        let filter = plugins.map(|p| p.get::<MetricFilter>()).unwrap_or(None);
        ensure!(
            config.write_interval.as_secs() != 0,
            InvalidExportMetricsConfigSnafu {
                msg: "Expected export metrics write_interval greater than zero"
            }
        );
        ensure!(
            !config.db.is_empty(),
            InvalidExportMetricsConfigSnafu {
                msg: "Expected export metrics db not empty"
            }
        );
        // construct http header
        let mut headers = reqwest::header::HeaderMap::with_capacity(config.headers.len());
        config.headers.iter().try_for_each(|(k, v)| {
            let header = match TryInto::<HeaderName>::try_into(k) {
                Ok(header) => header,
                Err(_) => {
                    return InvalidExportMetricsConfigSnafu {
                        msg: format!("Export metrics: invalid HTTP header name: {}", k),
                    }
                    .fail()
                }
            };
            match TryInto::<HeaderValue>::try_into(v) {
                Ok(value) => headers.insert(header, value),
                Err(_) => {
                    return InvalidExportMetricsConfigSnafu {
                        msg: format!("Export metrics: invalid HTTP header value: {}", v),
                    }
                    .fail()
                }
            };
            Ok(())
        })?;
        let same_addr = if let Some(addr) = http_addr {
            addr == config.endpoint.as_str()
        } else {
            false
        };
        // send by handler when remote write addr same with frontend http endpoint, or endpoint is not set
        let send_by_handler = same_addr || config.endpoint.is_empty();
        // `http_addr.is_none()` means is `datanode` or `metasrv`, can't omit `endpoint`
        if http_addr.is_none() && config.endpoint.is_empty() {
            return InvalidExportMetricsConfigSnafu {
                msg: "Export metrics: Missing `export_metrics.endpoint`. only `standalone` and `frontend` can omit it".to_string(),
            }
            .fail();
        }
        Ok(Some(Self {
            config: config.clone(),
            filter,
            headers,
            send_by_handler,
            handler: None,
        }))
    }

    pub fn set_handler(&mut self, handler: PromStoreProtocolHandlerRef) {
        self.handler = Some(handler);
    }

    pub fn start(&self) {
        if !self.config.enable {
            return;
        }
        let interval = time::interval(self.config.write_interval);
        let filter = self.filter.clone();
        let _handle = if let Some(h) = self.handler.clone() {
            common_runtime::spawn_bg(write_system_metric_by_handler(
                self.config.db.clone(),
                h,
                filter,
                interval,
            ))
        } else {
            common_runtime::spawn_bg(write_system_metric_by_network(
                self.headers.clone(),
                format!(
                    "http://{}/v1/prometheus/write?db={}",
                    self.config.endpoint, self.config.db
                ),
                filter,
                interval,
            ))
        };
    }
}

/// Send metrics collected by standard Prometheus [RemoteWrite format](https://prometheus.io/docs/concepts/remote_write_spec/)
pub async fn write_system_metric_by_network(
    headers: HeaderMap,
    endpoint: String,
    filter: Option<MetricFilter>,
    mut interval: Interval,
) {
    info!(
        "Start export metrics task to endpoint: {}, interval: {}s",
        endpoint,
        interval.period().as_secs()
    );
    // Pass the first tick. Because the first tick completes immediately.
    interval.tick().await;
    let client = reqwest::Client::new();
    loop {
        interval.tick().await;
        let metric_families = prometheus::gather();
        let request = convert_metric_to_write_request(
            metric_families,
            filter.as_ref(),
            Timestamp::current_millis().value(),
        );
        let resp = match snappy_compress(&request.encode_to_vec()) {
            Ok(body) => client
                .post(endpoint.as_str())
                .header("X-Prometheus-Remote-Write-Version", "0.1.0")
                .header("Content-Type", "application/x-protobuf")
                .headers(headers.clone())
                .body(body)
                .send()
                .await
                .context(SendPromRemoteRequestSnafu),
            Err(e) => Err(e),
        };
        match resp {
            Ok(resp) => {
                if !resp.status().is_success() {
                    error!("report export metrics error, msg: {:#?}", resp);
                }
            }
            Err(e) => error!("report export metrics failed, error {}", e),
        };
    }
}

/// Send metrics collected by our internal handler
/// for case `frontend` and `standalone` dispose it's own metrics,
/// reducing compression and network transmission overhead.
pub async fn write_system_metric_by_handler(
    db: String,
    handler: PromStoreProtocolHandlerRef,
    filter: Option<MetricFilter>,
    mut interval: Interval,
) {
    info!(
        "Start export metrics task by handler, interval: {}s",
        interval.period().as_secs()
    );
    // Pass the first tick. Because the first tick completes immediately.
    interval.tick().await;
    let ctx = QueryContextBuilder::default().current_schema(db).build();
    loop {
        interval.tick().await;
        let metric_families = prometheus::gather();
        let request = convert_metric_to_write_request(
            metric_families,
            filter.as_ref(),
            Timestamp::current_millis().value(),
        );
        if let Err(e) = handler.write(request, ctx.clone()).await {
            error!("report export metrics by handler failed, error {}", e);
        }
    }
}
