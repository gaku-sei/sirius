use leptos::html::Canvas as CanvasNode;
use leptos::{create_node_ref, NodeRef, Signal, SignalGet};
use leptos_use::{use_device_pixel_ratio, use_element_size, UseElementSizeReturn};

pub struct UseCanvasReturn {
    pub node_ref: NodeRef<CanvasNode>,
    pub dpr: Signal<f64>,
    pub width: Signal<f64>,
    pub height: Signal<f64>,
    pub actual_width: Signal<f64>,
    pub actual_height: Signal<f64>,
}

pub fn use_canvas() -> UseCanvasReturn {
    let node_ref = create_node_ref::<CanvasNode>();
    let dpr = use_device_pixel_ratio();
    let UseElementSizeReturn { width, height } = use_element_size(node_ref);
    let actual_width = move || width.get() * dpr.get();
    let actual_height = move || height.get() * dpr.get();

    UseCanvasReturn {
        node_ref,
        dpr,
        width,
        height,
        actual_width: actual_width.into(),
        actual_height: actual_height.into(),
    }
}
