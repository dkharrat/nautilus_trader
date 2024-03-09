// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2024 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use std::{collections::HashMap, fs, str::FromStr};

use anyhow::anyhow;
use databento::live::Subscription;
use indexmap::IndexMap;
use nautilus_core::{
    python::{to_pyruntime_err, to_pyvalue_err},
    time::UnixNanos,
};
use nautilus_model::{
    identifiers::{symbol::Symbol, venue::Venue},
    python::data::data_to_pycapsule,
};
use pyo3::prelude::*;
use time::OffsetDateTime;
use tracing::{debug, error, info};

use super::loader::convert_instrument_to_pyobject;
use crate::databento::{
    live::{DatabentoFeedHandler, LiveCommand, LiveMessage},
    types::DatabentoPublisher,
};

#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.databento")
)]
pub struct DatabentoLiveClient {
    #[pyo3(get)]
    pub key: String,
    #[pyo3(get)]
    pub dataset: String,
    #[pyo3(get)]
    pub is_running: bool,
    #[pyo3(get)]
    pub is_closed: bool,
    tx_cmd: std::sync::mpsc::Sender<LiveCommand>,
    rx_cmd: Option<std::sync::mpsc::Receiver<LiveCommand>>,
    publisher_venue_map: IndexMap<u16, Venue>,
    glbx_exchange_map: HashMap<Symbol, Venue>,
}

impl DatabentoLiveClient {
    async fn process_messages(
        mut rx: tokio::sync::mpsc::Receiver<LiveMessage>,
        callback: PyObject,
        callback_pyo3: PyObject,
    ) -> PyResult<()> {
        debug!("Processing messages...");
        // Continue to process messages until channel is hung up
        while let Some(msg) = rx.recv().await {
            match msg {
                LiveMessage::Data(data) => Python::with_gil(|py| {
                    let py_obj = data_to_pycapsule(py, data);
                    call_python(py, &callback, py_obj);
                }),
                LiveMessage::Instrument(data) => Python::with_gil(|py| {
                    let py_obj = convert_instrument_to_pyobject(py, data)
                        .expect("Error creating instrument");
                    call_python(py, &callback, py_obj);
                }),
                LiveMessage::Imbalance(data) => Python::with_gil(|py| {
                    let py_obj = data.into_py(py);
                    call_python(py, &callback_pyo3, py_obj);
                }),
                LiveMessage::Statistics(data) => Python::with_gil(|py| {
                    let py_obj = data.into_py(py);
                    call_python(py, &callback_pyo3, py_obj);
                }),
                LiveMessage::Error(e) => return Err(to_pyruntime_err(e)),
            };
        }
        Ok(())
    }

    fn send_command(&self, cmd: LiveCommand) -> PyResult<()> {
        self.tx_cmd.send(cmd).map_err(to_pyruntime_err)
    }
}

fn call_python(py: Python, callback: &PyObject, py_obj: PyObject) {
    match callback.call1(py, (py_obj,)) {
        Ok(_) => {}
        Err(e) => error!("Error calling Python: {e}"),
    }
}

#[pymethods]
impl DatabentoLiveClient {
    #[new]
    pub fn py_new(key: String, dataset: String, publishers_path: String) -> anyhow::Result<Self> {
        let file_content = fs::read_to_string(publishers_path)?;
        let publishers_vec: Vec<DatabentoPublisher> = serde_json::from_str(&file_content)?;

        let publisher_venue_map = publishers_vec
            .into_iter()
            .map(|p| (p.publisher_id, Venue::from(p.venue.as_str())))
            .collect::<IndexMap<u16, Venue>>();

        let (tx_cmd, rx_cmd) = std::sync::mpsc::channel::<LiveCommand>();

        Ok(Self {
            key,
            dataset,
            tx_cmd,
            rx_cmd: Some(rx_cmd),
            is_running: false,
            is_closed: false,
            publisher_venue_map,
            glbx_exchange_map: HashMap::new(),
        })
    }

    #[pyo3(name = "load_glbx_exchange_map")]
    fn py_load_glbx_exchange_map(&mut self, map: HashMap<Symbol, Venue>) -> PyResult<()> {
        self.glbx_exchange_map.clone_from(&map);
        self.send_command(LiveCommand::UpdateGlbx(map))
    }

    #[pyo3(name = "get_glbx_exchange_map")]
    fn py_get_glbx_exchange_map(&self) -> HashMap<Symbol, Venue> {
        self.glbx_exchange_map.clone()
    }

    #[pyo3(name = "subscribe")]
    fn py_subscribe(
        &mut self,
        schema: String,
        symbols: String,
        stype_in: Option<String>,
        start: Option<UnixNanos>,
    ) -> PyResult<()> {
        let stype_in = stype_in.unwrap_or("raw_symbol".to_string());

        let mut sub = Subscription::builder()
            .symbols(symbols)
            .schema(dbn::Schema::from_str(&schema).map_err(to_pyvalue_err)?)
            .stype_in(dbn::SType::from_str(&stype_in).map_err(to_pyvalue_err)?)
            .build();

        if let Some(start) = start {
            let start = OffsetDateTime::from_unix_timestamp_nanos(i128::from(start))
                .map_err(to_pyvalue_err)?;
            sub.start = Some(start);
        };

        self.send_command(LiveCommand::Subscribe(sub))
    }

    #[pyo3(name = "start")]
    fn py_start<'py>(
        &mut self,
        py: Python<'py>,
        callback: PyObject,
        callback_pyo3: PyObject,
    ) -> PyResult<&'py PyAny> {
        if self.is_closed {
            return Err(to_pyruntime_err("Client was already closed"));
        };
        if self.is_running {
            return Err(to_pyruntime_err("Client was already running"));
        };
        self.is_running = true;

        let (tx_msg, rx_msg) = tokio::sync::mpsc::channel::<LiveMessage>(100_000);

        // Consume the receiver
        let rx_cmd = self
            .rx_cmd
            .take()
            .ok_or_else(|| anyhow!("Client already started"))?;

        let mut feed_handler = DatabentoFeedHandler::new(
            self.key.clone(),
            self.dataset.clone(),
            rx_cmd,
            tx_msg,
            self.publisher_venue_map.clone(),
            HashMap::new(),
        );

        self.send_command(LiveCommand::Start)?;

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let feed_handle = tokio::task::spawn(async move { feed_handler.run().await });
            match Self::process_messages(rx_msg, callback, callback_pyo3).await {
                Ok(()) => debug!("Recv handler completed"),
                Err(e) => error!("Recv handler error: {e}"),
            }

            match feed_handle.await {
                Ok(result) => match result {
                    Ok(()) => info!("Feed handler completed"),
                    Err(e) => error!("Feed handler error: {e}"),
                },
                Err(e) => error!("Feed handler error: {e}"),
            }

            Ok(())
        })
    }

    #[pyo3(name = "close")]
    fn py_close(&mut self) -> PyResult<()> {
        if !self.is_running {
            return Err(to_pyruntime_err("Client was never started"));
        };

        self.send_command(LiveCommand::Close)?;

        self.is_running = false;
        self.is_closed = true;

        Ok(())
    }
}
