use std::{cmp::Ordering, f64::consts::PI, ops::Range};

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration, DurationRound, SecondsFormat, Utc};
use humantime::format_duration;
use leptos::{html::Canvas, HtmlElement};
use tracing::{debug, error, info};
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

use crate::binary_search::binary_search_by_with_index;

use super::types::{MeasureSet, MeasuresData};

const SCALE_PADDING: f64 = 16.0;

const COLORS: [&str; 5] = ["#ff00c1", "#9600ff", "#4900ff", "#00b8ff", "#00fff9"];

pub struct MeasuresCanvas {
    ctx: CanvasRenderingContext2d,
    force_debug: bool,
}

impl MeasuresCanvas {
    pub fn try_new(node: &HtmlElement<Canvas>) -> Result<Self> {
        #[derive(serde::Serialize)]
        struct ContextOptions {
            alpha: bool,
        }

        let ctx = node
            .get_context_with_context_options(
                "2d",
                &serde_wasm_bindgen::to_value(&ContextOptions { alpha: false })
                    .map_err(|err| anyhow!("context options serialization error: {err}"))?,
            )
            .map_err(|err| anyhow!("{err:?}"))?;
        let Some(ctx) = ctx else {
            bail!("canvas' 2d context not found");
        };

        let ctx = ctx
            .dyn_into()
            .map_err(|err| anyhow!("context dyn conversion error: {err:?}"))?;

        Ok(Self {
            ctx,
            force_debug: false,
        })
    }

    #[must_use]
    pub fn with_force_debug(mut self) -> Self {
        self.force_debug = true;
        self
    }

    #[expect(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        measures: &MeasuresData,
        begin: DateTime<Utc>,
        end: DateTime<Utc>,
        width: f64,
        height: f64,
        mouse_x: f64,
        dpr: f64,
    ) {
        debug!("rendering");

        if width < f64::EPSILON || height < f64::EPSILON {
            return;
        }

        self.ctx.save();
        if let Err(err) = self.ctx.scale(dpr, dpr) {
            error!(dpr, "context scaling failed: {err:?}");
        };
        self.ctx.set_font("14px Arial");
        self.ctx.set_fill_style(&"#13171f".into());
        self.ctx.fill_rect(0.0, 0.0, width, height);
        self.ctx.set_fill_style(&"white".into());

        self.render_scales(measures, width, height, begin, end, mouse_x);
        self.render_measures(measures, width, height, begin, end, mouse_x);
        self.render_dots(measures, width, height, begin, end, mouse_x);
        if self.force_debug || cfg!(debug_assertions) {
            self.render_stats(measures, width, height, begin, end, mouse_x);
        }

        self.ctx.restore();
    }

    fn render_scales(
        &mut self,
        _measures: &MeasuresData,
        width: f64,
        height: f64,
        begin: DateTime<Utc>,
        end: DateTime<Utc>,
        _mouse_x: f64,
    ) {
        debug!("rendering scales");

        let Some(begin_ns) = begin.timestamp_nanos_opt() else {
            error!(
                begin = begin.to_rfc3339(),
                "conversion nanoseconds overflow"
            );
            return;
        };
        let Some(end_ns) = end.timestamp_nanos_opt() else {
            error!(end = end.to_rfc3339(), "conversion nanoseconds overflow");
            return;
        };
        let y = height / 100.0 * 90.0;

        self.ctx.begin_path();
        self.ctx.move_to(0.0, y);
        self.ctx.line_to(width, y);
        self.ctx.set_stroke_style(&"white".into());
        self.ctx.stroke();

        let duration = end - begin;

        #[expect(clippy::cast_possible_truncation)]
        let scales = ((width / 204.0) as i32).max(1);

        let interval = duration / scales;

        let truncated_begin_time = match begin.duration_trunc(duration / scales) {
            Ok(truncated_begin_time) => truncated_begin_time,
            Err(err) => {
                error!(
                    begin = begin.to_rfc3339(),
                    %duration,
                    scales,
                    "date time duration trunc error: {err}"
                );
                return;
            }
        };

        let mut time = truncated_begin_time + duration / scales;
        for _ in 0..scales {
            let Some(time_ns) = time.timestamp_nanos_opt() else {
                error!(time = time.to_rfc3339(), "conversion nanoseconds overflow");
                continue;
            };
            let x = time_to_x(time_ns, begin_ns, end_ns, width);

            self.ctx.begin_path();
            self.ctx.move_to(x, y);
            self.ctx.line_to(x, y + SCALE_PADDING);
            self.ctx.set_stroke_style(&"white".into());
            self.ctx.stroke();

            if let Err(err) = self.ctx.fill_text(
                &time
                    .with_timezone(&chrono::Local)
                    .to_rfc3339_opts(SecondsFormat::Millis, true),
                x - 90.0,
                y + SCALE_PADDING * 2.0,
            ) {
                error!("fill text error: {err:?}");
            }

            time += interval;
        }
    }

    fn render_measures(
        &mut self,
        measures: &MeasuresData,
        width: f64,
        height: f64,
        begin: DateTime<Utc>,
        end: DateTime<Utc>,
        _mouse_x: f64,
    ) {
        debug!("rendering measures");

        let Some(begin_ns) = begin.timestamp_nanos_opt() else {
            error!(begin = begin.to_rfc3339(), "nanoseconds conversion error");
            return;
        };
        let Some(end_ns) = end.timestamp_nanos_opt() else {
            error!(end = end.to_rfc3339(), "nanoseconds conversion error");
            return;
        };

        for (index, (target, measure_set)) in measures.iter().enumerate() {
            info!("target={target}");

            self.ctx.begin_path();

            let color = get_color(index);
            self.ctx.set_stroke_style(&color.into());

            let max_measure = get_max_measure_value(measure_set, begin_ns, end_ns);

            for (index, (time, value)) in measure_set.measures.iter().enumerate() {
                let mut drawn = *time > begin_ns && *time < end_ns;

                if let Some((time, _value)) = measure_set.measures.get(index - 1) {
                    drawn |= *time > begin_ns && *time < end_ns;
                }

                if let Some((time, _value)) = measure_set.measures.get(index + 1) {
                    drawn |= *time > begin_ns && *time < end_ns;
                }

                if !drawn {
                    continue;
                }

                let x = time_to_x(*time, begin_ns, end_ns, width);
                let y = value_to_y(*value, max_measure, height);

                self.ctx.line_to(x, y);
            }

            self.ctx.stroke();
        }

        self.ctx.set_font("14px Arial");
        self.ctx.set_stroke_style(&"white".into());
    }

    fn render_dots(
        &mut self,
        measures: &MeasuresData,
        width: f64,
        height: f64,
        begin: DateTime<Utc>,
        end: DateTime<Utc>,
        mouse_x: f64,
    ) {
        debug!("rendering dots");

        let Some(begin_ns) = begin.timestamp_nanos_opt() else {
            error!(begin = begin.to_rfc3339(), "nanoseconds conversion error");
            return;
        };
        let Some(end_ns) = end.timestamp_nanos_opt() else {
            error!(end = end.to_rfc3339(), "nanoseconds conversion error");
            return;
        };
        let mouse_x_time = x_to_time(mouse_x, begin_ns, end_ns, width);

        for (index, measure_set) in measures.values().enumerate() {
            if let Some((time, value)) = find_closest_measure(&measure_set.measures, mouse_x_time) {
                let x = time_to_x(time, begin_ns, end_ns, width);
                let y = value_to_y(value, measure_set.max, height);

                let color = get_color(index);
                self.ctx.set_fill_style(&color.into());

                self.ctx.begin_path();
                if let Err(err) = self.ctx.arc(x, y, 2.0, 0.0, 2.0 * PI) {
                    error!("arc drawing error: {err:?}");
                }
                self.ctx.fill();

                self.ctx.set_fill_style(&"white".into());
            }
        }
    }

    #[expect(clippy::too_many_lines, clippy::cast_precision_loss)]
    fn render_stats(
        &mut self,
        measures: &MeasuresData,
        _width: f64,
        _height: f64,
        begin: DateTime<Utc>,
        end: DateTime<Utc>,
        _mouse_x: f64,
    ) {
        debug!("rendering stats");

        let Some(begin_ns) = begin.timestamp_nanos_opt() else {
            error!(begin = begin.to_rfc3339(), "nanoseconds conversion error");
            return;
        };
        let Some(end_ns) = end.timestamp_nanos_opt() else {
            error!(end = end.to_rfc3339(), "nanoseconds conversion error");
            return;
        };
        let duration = end - begin;

        let mut num_points = 0;

        for measure_set in measures.values() {
            for (index, (time, _value)) in measure_set.measures.iter().enumerate() {
                let mut displayed = *time > begin_ns && *time < end_ns;

                if let Some((time, _value)) = measure_set.measures.get(index - 1) {
                    displayed |= *time > begin_ns && *time < end_ns;
                }

                if let Some((time, _value)) = measure_set.measures.get(index + 1) {
                    displayed |= *time > begin_ns && *time < end_ns;
                }

                if !displayed {
                    continue;
                }

                num_points += 1;
            }
        }

        let num_points = num_points.to_string();
        let num_points = num_points
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>();

        let num_points = match num_points {
            Ok(num_points) => num_points.join(","),
            Err(err) => {
                error!("utf8 conversion error: {err}");
                "unknown".to_string()
            }
        };

        if let Err(err) = self
            .ctx
            .fill_text(&format!("rendering {num_points} points"), 16.0, 16.0)
        {
            error!("fill text error: {err:?}");
        }

        match duration.to_std() {
            Ok(duration) => {
                if let Err(err) = self.ctx.fill_text(
                    &format!("duration {}", format_duration(duration)),
                    16.0,
                    32.0,
                ) {
                    error!("fill text error: {err:?}");
                }
            }
            Err(err) => {
                error!(
                    %duration,
                    "duration couldn't be converted to std duration: {err:?}"
                );
            }
        }

        if let Err(err) = self
            .ctx
            .fill_text(&format!("lod {}", compute_lod(duration)), 16.0, 48.0)
        {
            error!("fill text error: {err:?}");
        }

        let segment_duration = compute_segment_duration(compute_lod(duration));
        match segment_duration.to_std() {
            Ok(segment_duration) => {
                if let Err(err) = self.ctx.fill_text(
                    &format!("segment duration {}", format_duration(segment_duration)),
                    16.0,
                    64.0,
                ) {
                    error!("fill text error: {err:?}");
                }
            }
            Err(err) => {
                error!(
                    %segment_duration,
                    "duration couldn't be converted to std duration: {err:?}"
                );
            }
        }

        if let Some(segments) = compute_segment_index(begin, end, compute_lod(duration)) {
            if let Err(err) = self.ctx.fill_text(
                &format!("first={} last={}", segments.start, segments.end),
                16.0,
                80.0,
            ) {
                error!("fill text error: {err:?}");
            }
        } else if let Err(err) = self.ctx.fill_text("unknown segments", 16.0, 80.0) {
            error!("fill text error: {err:?}");
        }

        for (index, (target, measure_set)) in measures.iter().enumerate() {
            let color = get_color(index);
            self.ctx.set_fill_style(&color.into());
            if let Err(err) = self.ctx.fill_text(
                &format!("{target} ({})", measure_set.unit),
                16.0,
                96.0 + (index as f64 * 16.0),
            ) {
                error!("fill text error: {err:?}");
            }
            self.ctx.set_fill_style(&"white".into());
        }
    }
}

#[expect(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
pub fn compute_lod(duration: Duration) -> u32 {
    // ((duration.num_seconds() as f64).log10() - 1.0) as u32
    // ((duration.num_seconds() as f64).log(5.0)).max(0.0) as u32
    ((duration.num_milliseconds() as f64).log10() - 2.0).max(0.0) as u32
}

fn compute_segment_duration(lod: u32) -> Duration {
    Duration::milliseconds(10_i64.pow(lod + 3) / 10)
}

fn compute_segment_index(begin: DateTime<Utc>, end: DateTime<Utc>, lod: u32) -> Option<Range<i64>> {
    let segment_duration = compute_segment_duration(lod);
    let first =
        (begin - DateTime::UNIX_EPOCH).num_nanoseconds()? / segment_duration.num_nanoseconds()?;
    let last =
        (end - DateTime::UNIX_EPOCH).num_nanoseconds()? / segment_duration.num_nanoseconds()?;
    Some(first..last)
}

#[expect(clippy::cast_precision_loss)]
fn time_to_x(time: i64, begin_ns: i64, end_ns: i64, width: f64) -> f64 {
    let rev_factor = 1.0 / (end_ns - begin_ns) as f64;
    let delta = (time - begin_ns) as f64;
    rev_factor * delta * width
}

#[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn x_to_time(x: f64, begin_ns: i64, end_ns: i64, width: f64) -> i64 {
    let rev_factor = 1.0 / width;
    begin_ns + (rev_factor * x * (end_ns - begin_ns) as f64) as i64
}

fn value_to_y(value: f64, max_value: f64, height: f64) -> f64 {
    let rev_factor = 1.0 / max_value;
    let y_margin = y_margin(height);
    height - rev_factor * value * (height - y_margin) - y_margin
}

fn y_margin(height: f64) -> f64 {
    height / 10.0
}

pub fn get_color(index: usize) -> &'static str {
    COLORS[index % COLORS.len()]
}

pub fn get_max_measure_value(measure_set: &MeasureSet, begin_ns: i64, end_ns: i64) -> f64 {
    let mut displayed_values = Vec::with_capacity(4 * 1024);

    for (index, (time, value)) in measure_set.measures.iter().enumerate() {
        let mut displayed = *time > begin_ns && *time < end_ns;

        if let Some((time, _value)) = measure_set.measures.get(index - 1) {
            displayed |= *time > begin_ns && *time < end_ns;
        }

        if let Some((time, _value)) = measure_set.measures.get(index + 1) {
            displayed |= *time > begin_ns && *time < end_ns;
        }

        if !displayed {
            continue;
        }

        displayed_values.push(*value);
    }

    find_max_measure_value(&displayed_values).unwrap_or(measure_set.max)
}

pub fn find_max_measure_value(measures: &[f64]) -> Option<f64> {
    measures
        .iter()
        .max_by(|value1, value2| value1.total_cmp(value2))
        .copied()
}

pub fn find_closest_measure(measures: &[(i64, f64)], mouse_x_time: i64) -> Option<(i64, f64)> {
    let res = binary_search_by_with_index(measures, |index, (time, _value)| {
        if let Some((prev_time, _)) = measures.get(index - 1) {
            if *time > mouse_x_time && *prev_time < mouse_x_time {
                return Ordering::Equal;
            }
        }

        if *time < mouse_x_time {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    });

    let index = match res {
        Ok(x) | Err(x) => x,
    };

    measures.get(index).copied()
}
