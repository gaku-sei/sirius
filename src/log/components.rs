use chrono::{DateTime, Duration, Utc};
use leptos::{
    component, create_effect, create_memo, view, For, IntoView, Params, SignalGet, SignalWith,
};
use leptos_router::{use_params, Params};
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::{
    components::Spinner,
    datetime::display_datetime,
    request::{create_request, QueryRequest, QUERY_URL},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: String,
    pub target: String,
    pub msg: String,
}

#[derive(Params, PartialEq)]
struct LogParams {
    id: Option<Uuid>,
}

#[component]
pub fn Log() -> impl IntoView {
    let params = use_params::<LogParams>();
    let id = move || {
        params.with(|params| {
            params
                .as_ref()
                .ok()
                .and_then(|params| params.id)
                .unwrap_or_default()
        })
    };

    let request = create_memo(move |_| log_request(id()));
    let log = create_request::<_, Vec<LogEntry>>(move || request.get());

    create_effect(move |_| {
        let count = log
            .get()
            .map(|log_entries| log_entries.len())
            .unwrap_or_default();

        debug!("count={count}");
    });

    view! {
        <div class="w-full p-4 flex flex-col items-center">
            {move || {
                if log.loading().get() {
                    view! { <Spinner /> }
                } else {
                    let log_entries = log.get().unwrap_or_default();
                    view! { <LogEntries log_entries></LogEntries> }
                }
            }}
        </div>
    }
}

#[component]
pub fn LogEntries(log_entries: Vec<LogEntry>) -> impl IntoView {
    view! {
        <div class="overflow-auto">
            <table class="striped">
                <thead>
                    <tr>
                        <th scope="col">"Time"</th>
                        <th scope="col">"Target"</th>
                        <th scope="col">"Message"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || log_entries.clone()
                        key=|log_entry| log_entry.time.clone()
                        let:log_entry
                    >
                        <LogEntryRow log_entry></LogEntryRow>
                    </For>
                </tbody>
                <tfoot>
                    <tr>
                        <th scope="row">Average</th>
                        <td>9,126</td>
                        <td>0.91</td>
                        <td>341</td>
                    </tr>
                </tfoot>
            </table>
        </div>
    }
}

#[component]
pub fn LogEntryRow(log_entry: LogEntry) -> impl IntoView {
    let datetime = DateTime::parse_from_rfc3339(&log_entry.time)
        .ok()
        .map_or_else(
            || "invalid date time".to_string(),
            |datetime| display_datetime(datetime.into()),
        );

    view! {
        <tr>
            <td>{datetime}</td>
            <td>{log_entry.target}</td>
            <td>{log_entry.msg}</td>
        </tr>
    }
}

fn log_request(id: Uuid) -> (String, QueryRequest) {
    let end = Utc::now();
    let begin = end - Duration::days(1);
    let request = QueryRequest {
        sql: format!(
            "select * from log_entries where process_id = '{id}' order by time desc limit 5000"
        ),
        begin: Some(begin.to_rfc3339()),
        end: Some(end.to_rfc3339()),
    };

    (QUERY_URL.to_string(), request)
}
