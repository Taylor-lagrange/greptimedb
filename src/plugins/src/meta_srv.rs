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

use common_base::Plugins;
use meta_srv::error::{InitExportMetricsTaskSnafu, Result};
use meta_srv::metasrv::MetaSrvOptions;
use servers::export_metrics::ExportMetricsTask;
use snafu::ResultExt;

pub async fn setup_meta_srv_plugins(opts: &mut MetaSrvOptions) -> Result<Plugins> {
    let plugins = Plugins::new();
    if let Some(export_metrics_task) =
        ExportMetricsTask::try_new(&opts.export_metrics, None, Some(&plugins))
            .context(InitExportMetricsTaskSnafu)?
    {
        plugins.insert(export_metrics_task);
    }
    Ok(plugins)
}

pub async fn start_meta_srv_plugins(plugins: Plugins) -> Result<()> {
    if let Some(export_metrics_task) = plugins.get::<ExportMetricsTask>() {
        export_metrics_task.start();
    }
    Ok(())
}
