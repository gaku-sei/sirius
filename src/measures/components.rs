use core::f64;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ev::{MouseEvent, WheelEvent};
use leptos::html::Article;
use leptos::html::Canvas as CanvasNode;
use leptos::{
    component, create_memo, create_node_ref, ev, view, For, IntoView, NodeRef, Signal, SignalGet,
    SignalSet, SignalUpdate,
};
use tracing::error;

use crate::components::Spinner;
use crate::datetime::display_datetime;

use super::canvas::{find_closest_measure, get_color, x_to_time};
use super::create_measures::{create_measures, CreateMeasuresReturn};
use super::types::MeasureSet;

// TODO: Group by topic instead of a big struct
#[component]
pub fn Measures() -> impl IntoView {
    let CreateMeasuresReturn {
        canvas_node_ref,

        set_dragging,
        set_mouse_x,
        set_mouse_y,
        set_begin,
        set_end,
        set_canvas_has_focus,

        is_loading,
        is_dragging,
        canvas_width,
        mouse_x,
        mouse_y,
        window_width,
        duration,
        begin,
        end,
        measures,
        measures_targets,
        canvas_has_focus,
        canvas_height,
        canvas_actual_width,
        canvas_actual_height,
    } = create_measures();

    let handle_mousemove = move |evt: MouseEvent| {
        let Some(canvas_node) = canvas_node_ref.get() else {
            return;
        };

        let dom_rect = canvas_node.get_bounding_client_rect();

        set_mouse_x.set(f64::from(evt.client_x()) - dom_rect.left());
        set_mouse_y.set(f64::from(evt.client_y()) - dom_rect.top());

        if !is_dragging.get() {
            return;
        }

        let factor = canvas_width.get() / f64::from(evt.movement_x());
        // TODO: Keep an eye on this cast
        #[expect(clippy::cast_possible_truncation)]
        let delta = duration.get() / factor as i32;

        set_begin.update(|begin| *begin -= delta);
        set_end.update(|end| *end -= delta);
    };

    let handle_mouseenter = move |_evt: MouseEvent| {
        set_canvas_has_focus.set(true);
    };

    let handle_mouseleave = move |_evt: MouseEvent| {
        set_canvas_has_focus.set(false);
    };

    let handle_wheel = move |evt: WheelEvent| {
        let mut duration = duration.get() / 1000;
        if evt.delta_y() < 0.0 {
            duration = -duration;
        }

        let rev_factor = 1.0 / canvas_width.get();
        // TODO: Keep an eye on this cast
        #[expect(clippy::cast_possible_truncation)]
        let x_delta = ((rev_factor * f64::from(evt.x())) * 100.0) as i32;

        set_begin.update(|begin| *begin -= duration * x_delta);
        set_end.update(|end| *end += duration * (100 - x_delta));
    };

    view! {
        <div
            class="w-full h-full relative"
            on:mousedown=move |_| set_dragging.set(true)
            on:mouseup=move |_| set_dragging.set(false)
            on:mousemove=handle_mousemove
            on:mouseenter=handle_mouseenter
            on:mouseleave=handle_mouseleave
            on:wheel=handle_wheel
        >
            <div
                class="w-full h-full flex justify-center items-center"
                class:hidden=move || !is_loading.get()
            >

                // canvas.get().is_some() && measures.get().is_some()
                <Spinner />
            </div>

            // <MetricsDrowpdown measures_targets=measures_targets></MetricsDrowpdown>

            <Tooltip
                mouse_x
                mouse_y
                window_width
                canvas_height
                canvas_width
                begin
                end
                measures
                canvas_has_focus
                measures_targets
            ></Tooltip>

            <Canvas
                canvas_node_ref
                actual_width=canvas_actual_width
                actual_height=canvas_actual_height
                is_loading
            ></Canvas>
        </div>
    }
}

// TODO: Finish dropdown
#[component]
fn MetricsDrowpdown(measures_targets: Signal<Option<Vec<(String, String)>>>) -> impl IntoView {
    view! {
        <details class="dropdown">
            <summary>Dropdown</summary>
            <ul>
                <For
                    each=move || measures_targets.get().unwrap_or_default()
                    key=|(target, _)| target.clone()
                    let:target
                >
                    <li>
                        <label>
                            <input type="checkbox" name=target.0.clone() />
                            {target.0}
                        </label>

                    </li>
                </For>
            </ul>
        </details>
    }
}

#[component]
fn Canvas(
    canvas_node_ref: NodeRef<CanvasNode>,
    actual_width: Signal<f64>,
    actual_height: Signal<f64>,
    is_loading: Signal<bool>,
) -> impl IntoView {
    view! {
        <canvas
            class="border border-black w-full h-full"
            class:hidden=move || is_loading.get()
            node_ref=canvas_node_ref
            width=actual_width
            height=actual_height
            style:width="100%"
            style:height="600px"
        />
    }
}

#[expect(clippy::similar_names)]
#[component]
fn Tooltip(
    mouse_x: Signal<f64>,
    mouse_y: Signal<f64>,
    window_width: Signal<f64>,
    canvas_height: Signal<f64>,
    canvas_width: Signal<f64>,
    begin: Signal<DateTime<Utc>>,
    end: Signal<DateTime<Utc>>,
    canvas_has_focus: Signal<bool>,
    measures: Signal<Option<HashMap<String, MeasureSet>>>,
    measures_targets: Signal<Option<Vec<(String, String)>>>,
) -> impl IntoView {
    let tooltip_node_ref = create_node_ref::<Article>();

    let tooltip_x_position = move || {
        let mouse_x = mouse_x.get();

        let Some(tooltip) = tooltip_node_ref.get() else {
            return mouse_x + 8.0;
        };

        (window_width.get() - f64::from(tooltip.client_width()) - 64.0).min(mouse_x + 8.0)
    };

    let tooltip_y_position = move || {
        let mouse_y = mouse_y.get();

        let Some(tooltip) = tooltip_node_ref.get() else {
            return mouse_y + 8.0;
        };

        if mouse_y > canvas_height.get() * 0.7 {
            mouse_y - f64::from(tooltip.client_height()) - 8.0
        } else {
            mouse_y + 8.0
        }
    };

    let current_time = create_memo(move |_| {
        let Some(begin_ns) = begin.get().timestamp_nanos_opt() else {
            error!(
                begin = begin.get().to_rfc3339(),
                "conversion to nanoseconds overflow"
            );
            return None;
        };
        let Some(end_ns) = end.get().timestamp_nanos_opt() else {
            error!(
                end = end.get().to_rfc3339(),
                "conversion to nanoseconds overflow"
            );
            return None;
        };

        Some(DateTime::from_timestamp_nanos(x_to_time(
            mouse_x.get(),
            begin_ns,
            end_ns,
            canvas_width.get(),
        )))
    });

    view! {
        <article
            node_ref=tooltip_node_ref
            class="absolute flex flex-col"
            class:hidden=move || !canvas_has_focus.get()
            style:left=move || format!("{}px", tooltip_x_position())
            style:top=move || format!("{}px", tooltip_y_position())
        >
            {move || {
                current_time
                    .get()
                    .map(|current_time| {
                        view! { <div>{display_datetime(current_time)}</div> }.into_view()
                    })
                    .unwrap_or_default()
            }}
            <For
                each=move || measures_targets.get().unwrap_or_default().into_iter().enumerate()
                key=|(_, (target, _))| target.clone()
                children=move |(index, (target, count))| {
                    let target_ = target.clone();
                    let value = create_memo(move |_| {
                        let current_time = current_time.get()?;
                        let measures = measures.get()?;
                        let measures_set = measures.get(&target_)?;
                        let Some(current_time) = current_time.timestamp_nanos_opt() else {
                            error!(
                                current_time=current_time.to_rfc3339(), "conversion to nanoseconds overflow"
                            );
                            return None;
                        };
                        find_closest_measure(&measures_set.measures, current_time)
                            .map(|(_time, value)| value)
                    });
                    view! {
                        <div>
                            {target} ": " {move || value.get().unwrap_or(0.0)} " ("
                            <span style:color=move || get_color(index)>{count}</span> ")"
                        </div>
                    }
                }
            />
        </article>
    }
}
