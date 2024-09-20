use std::ffi::OsStr;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use leptos::{component, view, For, IntoView, SignalGet};

use crate::components::Spinner;
use crate::datetime::display_datetime;
use crate::request::{create_request, QueryRequest, QUERY_URL};
use crate::types::ProcessInfo;

#[component]
pub fn Home() -> impl IntoView {
    let processes = create_request::<_, Vec<ProcessInfo>>(processes_request);

    view! {
        <div class="overflow-auto">
            <table class="striped">
                <thead>
                    <tr>
                        <th scope="col"></th>
                        <th scope="col">"ID"</th>
                        <th scope="col">"Exe"</th>
                        <th scope="col">"Start time"</th>
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        if processes.loading().get() {
                            view! { <Spinner /> }
                        } else {
                            let processes = processes.get().unwrap_or_default();
                            view! {
                                <For
                                    each=move || processes.clone()
                                    key=|process| process.process_id.clone()
                                    let:process
                                >
                                    <Process process=process></Process>
                                </For>
                            }
                        }
                    }}
                </tbody>
            </table>
        </div>
    }
}

#[component]
pub fn Process(process: ProcessInfo) -> impl IntoView {
    let exe_path = PathBuf::from(process.exe);
    let exe = exe_path
        .file_name()
        .map_or_else(|| "unknown".into(), OsStr::to_string_lossy);
    let exe = exe.into_owned();
    let start_time = display_datetime(process.start_time);

    view! {
        <tr>
            <th scope="row">
                <a href=format!("/measures/{}", process.process_id)>"Measures"</a>
                " / "
                <a href=format!("/log/{}", process.process_id)>"Log"</a>
            </th>
            <td>{process.process_id}</td>
            <td>{exe}</td>
            <td>{start_time}</td>
        </tr>
    }
}

fn processes_request() -> (String, QueryRequest) {
    let end = Utc::now();
    let begin = end - Duration::days(1);
    let request = QueryRequest {
        sql: "select * from processes order by start_time desc limit 100".to_string(),
        begin: Some(begin.to_rfc3339()),
        end: Some(end.to_rfc3339()),
    };

    (QUERY_URL.to_string(), request)
}
