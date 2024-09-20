use std::io::Cursor;

use anyhow::{anyhow, Result};
use arrow_json::ArrayWriter;
use leptos::{create_resource, Resource, Serializable};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::Serialize;
use tracing::{debug, error};

pub const FIND_PROCESS_URL: &str = "http://localhost:8082/analytics/find_process";
pub const QUERY_URL: &str = "http://localhost:8082/analytics/query";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FindProcessRequest {
    // Sending a string instead of an Uuid for compatibility purpose with the API
    pub process_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QueryRequest {
    pub begin: Option<String>,
    pub end: Option<String>,
    pub sql: String,
}

pub fn create_request<Request, T>(
    request: impl Fn() -> (String, Request) + 'static,
) -> Resource<(String, Request), T>
where
    Request: PartialEq + Clone + Serialize + 'static,
    // TODO: Temporarily handle errors in this function using the default value of the expected type.
    // In the long run this constraint should be lifted.
    T: Serializable + Default + 'static,
{
    create_resource(request, move |(url, request)| async move {
        match perform_request(&url, &request).await {
            Ok(value) => value,
            Err(err) => {
                // TODO: Return an error instead of this
                error!("request error: {err}");
                T::default()
            }
        }
    })
}

pub fn create_request_opt<Request, T>(
    request: impl Fn() -> Option<(String, Request)> + 'static,
) -> Resource<Option<(String, Request)>, T>
where
    Request: PartialEq + Clone + Serialize + 'static,
    T: Serializable + Default + 'static,
{
    create_resource(request, move |request| async move {
        if let Some((url, request)) = request {
            // TODO: Return an error instead of this
            match perform_request(&url, &request).await {
                Ok(value) => value,
                Err(err) => {
                    error!("request error: {err}");
                    T::default()
                }
            }
        } else {
            T::default()
        }
    })
}

async fn perform_request<R: Serializable>(url: &str, request: &impl Serialize) -> Result<R> {
    let mut buffer = Vec::new();
    let writer = Cursor::new(&mut buffer);
    ciborium::into_writer(&request, writer)?;

    let client = reqwest::Client::new();
    let response = client.post(url).body(buffer).send().await?;
    let bytes = response.bytes().await?;

    let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;

    let batches = reader
        .map(|res| res.map_err(Into::into))
        .collect::<Result<Vec<_>>>()?;

    let mut writer = ArrayWriter::new(Vec::new());
    writer.write_batches(&batches.iter().collect::<Vec<_>>())?;
    writer.finish()?;

    let json = String::from_utf8(writer.into_inner())?;

    debug!("json={json}");

    R::de(&json).map_err(|err| anyhow!("deserialization error: {err}"))
}
