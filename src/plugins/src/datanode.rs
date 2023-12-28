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

use std::sync::Arc;

use common_base::Plugins;
use common_greptimedb_telemetry::GreptimeDBTelemetryTask;
use common_telemetry::warn;
use datanode::config::DatanodeOptions;
use datanode::error::{Result, StartServerSnafu};
use datanode::greptimedb_telemetry::get_greptimedb_telemetry_task;
use servers::export_metrics::ExportMetricsTask;
use snafu::ResultExt;

pub async fn setup_datanode_plugins(opts: &mut DatanodeOptions) -> Result<Plugins> {
    let plugins = Plugins::new();
    if let Some(export_metrics_task) =
        ExportMetricsTask::try_new(&opts.export_metrics, None, Some(&plugins))
            .context(StartServerSnafu)?
    {
        plugins.insert(export_metrics_task);
    }
    plugins.insert(
        get_greptimedb_telemetry_task(
            Some(opts.storage.data_home.clone()),
            &opts.mode,
            opts.enable_telemetry,
        )
        .await,
    );
    Ok(plugins)
}

pub async fn start_datanode_plugins(plugins: Plugins) -> Result<()> {
    if let Some(export_metrics_task) = plugins.get::<ExportMetricsTask>() {
        export_metrics_task.start();
    }
    if let Some(greptimedb_telemetry_task) = plugins.get::<Arc<GreptimeDBTelemetryTask>>() {
        if let Err(e) = greptimedb_telemetry_task.start() {
            warn!(e; "Failed to start telemetry task!");
        }
    }
    Ok(())
}
