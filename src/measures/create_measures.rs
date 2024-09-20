use std::{cell::RefCell, collections::HashMap, rc::Rc};

use chrono::{DateTime, Duration, Utc};
use leptos::{
    create_effect, create_memo, create_signal, html::Canvas, NodeRef, Params, Resource, Signal,
    SignalGet, SignalSet, SignalWith, WriteSignal,
};
use leptos_router::{use_params, Params};
use leptos_use::{use_window_size, UseWindowSizeReturn};
use tracing::error;
use uuid::Uuid;

use crate::{
    request::{
        create_request, create_request_opt, FindProcessRequest, QueryRequest, FIND_PROCESS_URL,
        QUERY_URL,
    },
    types::ProcessInfo,
    use_canvas::{use_canvas, UseCanvasReturn},
};

use super::{
    canvas::MeasuresCanvas,
    types::{Measure, MeasureSet, MeasuresData},
};

pub struct CreateMeasuresReturn {
    pub canvas_node_ref: NodeRef<Canvas>,

    pub set_dragging: WriteSignal<bool>,
    pub set_mouse_x: WriteSignal<f64>,
    pub set_mouse_y: WriteSignal<f64>,
    pub set_begin: WriteSignal<DateTime<Utc>>,
    pub set_end: WriteSignal<DateTime<Utc>>,
    pub set_canvas_has_focus: WriteSignal<bool>,

    pub is_loading: Signal<bool>,
    pub is_dragging: Signal<bool>,
    pub canvas_width: Signal<f64>,
    pub mouse_x: Signal<f64>,
    pub mouse_y: Signal<f64>,
    pub window_width: Signal<f64>,
    pub duration: Signal<Duration>,
    pub begin: Signal<DateTime<Utc>>,
    pub end: Signal<DateTime<Utc>>,
    pub measures: Signal<Option<HashMap<String, MeasureSet>>>,
    pub measures_targets: Signal<Option<Vec<(String, String)>>>,
    pub canvas_has_focus: Signal<bool>,
    pub canvas_height: Signal<f64>,
    pub canvas_actual_width: Signal<f64>,
    pub canvas_actual_height: Signal<f64>,
}

pub fn create_measures() -> CreateMeasuresReturn {
    let id = use_params_id();

    let UseWindowSizeReturn {
        width: window_width,
        ..
    } = use_window_size();

    let UseCanvasReturn {
        node_ref: canvas_node_ref,
        dpr,
        width: canvas_width,
        height: canvas_height,
        actual_width: canvas_actual_width,
        actual_height: canvas_actual_height,
    } = use_canvas();

    let (canvas, set_canvas) = create_signal::<Option<Rc<RefCell<MeasuresCanvas>>>>(None);
    let (is_dragging, set_dragging) = create_signal(false);
    let (mouse_x, set_mouse_x) = create_signal(0.0);
    let (mouse_y, set_mouse_y) = create_signal(0.0);

    let (begin, set_begin) = create_signal(Utc::now() - Duration::hours(24));
    let (end, set_end) = create_signal(begin.get() + Duration::hours(25));
    let duration = move || end.get() - begin.get();

    let (canvas_has_focus, set_canvas_has_focus) = create_signal(false);

    let processes = create_request::<_, Vec<ProcessInfo>>(move || processes_request(id.get()));

    let measures_resource = create_request_opt::<_, Option<Vec<Measure>>>(move || {
        let processes = processes.get()?;
        let process = processes.first()?;

        Some(measures_request(process))
    });

    let measures = create_measures_memo(measures_resource);
    let measures_targets = create_measures_targets_memo(measures);

    let is_loading = move || processes.loading().get() || measures_resource.loading().get();

    create_effect(move |_| {
        let Some(processes) = processes.get() else {
            return;
        };

        let Some(process) = processes.first() else {
            return;
        };

        set_begin.set(process.start_time);
        set_end.set(Utc::now());
    });

    create_effect(move |_| {
        let Some(node) = canvas_node_ref.get() else {
            return;
        };

        let measures_canvas = match MeasuresCanvas::try_new(&node) {
            Ok(measures_canvas) => measures_canvas,
            Err(err) => {
                error!("measures canvas failed to initialize: {err}");
                return;
            }
        };

        // TODO: Remove force debug when more stable
        let canvas = Rc::new(RefCell::new(measures_canvas.with_force_debug()));
        set_canvas.set(Some(canvas));
    });

    create_effect(move |_| {
        canvas_width.track();
        canvas_height.track();

        let Some(canvas) = canvas.get() else {
            return;
        };

        let measures = measures.get().unwrap_or_default();

        canvas.borrow_mut().render(
            &measures,
            begin.get(),
            end.get(),
            canvas_width.get(),
            canvas_height.get(),
            mouse_x.get(),
            dpr.get(),
        );
    });

    CreateMeasuresReturn {
        canvas_node_ref,

        set_dragging,
        set_mouse_x,
        set_mouse_y,
        set_begin,
        set_end,
        set_canvas_has_focus,

        is_loading: is_loading.into(),
        is_dragging: is_dragging.into(),
        canvas_width,
        mouse_x: mouse_x.into(),
        mouse_y: mouse_y.into(),
        window_width,
        duration: duration.into(),
        begin: begin.into(),
        end: end.into(),
        measures,
        measures_targets,
        canvas_has_focus: canvas_has_focus.into(),
        canvas_height,
        canvas_actual_width,
        canvas_actual_height,
    }
}

#[derive(Params, PartialEq)]
struct MeasuresParams {
    id: Option<Uuid>,
}

fn use_params_id() -> Signal<Uuid> {
    let params = use_params::<MeasuresParams>();
    let id = move || {
        params.with(|params| {
            params
                .as_ref()
                .ok()
                .and_then(|params| params.id)
                .unwrap_or_default()
        })
    };
    id.into()
}

// TODO: Use Arrow and replace this function by a proper query in memory
// Use the same technique for data dissemination (lod)
fn create_measures_memo(
    measures: Resource<Option<(String, QueryRequest)>, Option<Vec<Measure>>>,
) -> Signal<Option<HashMap<String, MeasureSet>>> {
    create_memo(move |_| {
        let measures = measures.get().flatten()?;

        let mut measures_data: MeasuresData = HashMap::new();
        for measure in measures {
            let datetime = match DateTime::parse_from_rfc3339(&measure.time) {
                Ok(time) => time,
                Err(err) => {
                    error!(measure.time, "datetime parse error: {err}");
                    continue;
                }
            };

            let Some(time) = datetime.timestamp_nanos_opt() else {
                error!(measure.time, "conversion to nanoseconds overflow");
                continue;
            };

            let value = measure.value;

            measures_data
                .entry(measure.target)
                .and_modify(|measure_set| {
                    measure_set.min = measure_set.min.min(value);
                    measure_set.max = measure_set.max.max(value);
                    measure_set.start = measure_set.start.min(time);
                    measure_set.end = measure_set.end.max(time);
                    measure_set.measures.push((time, value));
                })
                .or_insert(MeasureSet {
                    min: value,
                    max: value,
                    start: time,
                    end: time,
                    // Assuming the unit never changes for any given target
                    unit: measure.unit,
                    measures: vec![(time, value)],
                });
        }

        Some(measures_data)
    })
    .into()
}

fn create_measures_targets_memo(
    measures: Signal<Option<HashMap<String, MeasureSet>>>,
) -> Signal<Option<Vec<(String, String)>>> {
    create_memo(move |_| {
        let measures = measures.get()?;

        let measure_targets = measures
            .iter()
            .map(|(target, measure_set)| (target.clone(), measure_set.unit.clone()))
            .collect::<Vec<_>>();

        Some(measure_targets)
    })
    .into()
}

fn processes_request(process_id: Uuid) -> (String, FindProcessRequest) {
    (
        FIND_PROCESS_URL.to_string(),
        FindProcessRequest {
            process_id: process_id.to_string(),
        },
    )
}

fn measures_request(process: &ProcessInfo) -> (String, QueryRequest) {
    let begin = process.start_time;
    let end = Utc::now();
    let request = QueryRequest {
        sql: format!(
            "
                SELECT target, time, value, unit
                  FROM measures
                 WHERE process_id = '{}'
                 ORDER BY time asc
            ",
            process.process_id
        ),
        begin: Some(begin.to_rfc3339()),
        end: Some(end.to_rfc3339()),
    };

    (QUERY_URL.to_string(), request)
}
