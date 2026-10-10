//! Graph view: multi-function input, coordinate system with grid,
//! zoom/pan controls, canvas plot.
//!
//! Sampling runs in `commands::sample_graph_points` (engine API); zoom/pan
//! only re-derive the sample range. This view renders stored [`GraphPoint`]s;
//! discontinuities (`valid: false`) break polylines.

use iced::mouse::Cursor;
use iced::widget::canvas::{Action, Canvas, Event, Frame, Geometry, Path, Program, Stroke};
use iced::widget::{button, column, row, text, text_input};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Theme};

use crate::messages::Message;
use crate::ods_theme::{subtle_button, ACCENT_PRESETS};
use crate::state::AppState;

/// Canvas padding shared by drawing and hover mapping (logical pixels).
pub const CANVAS_PAD: f32 = 12.0;

/// Vertical data extents across all valid samples, with the same fallback,
/// minimum-span guard and 10% margins as the canvas renderer.
///
/// Returns `(ymin, ymax)` in data coordinates.
#[must_use]
pub fn graph_y_extents(series: &[GraphSeries]) -> (f64, f64) {
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;
    for s in series {
        for p in s.samples.iter().filter(|p| p.valid) {
            ymin = ymin.min(p.y);
            ymax = ymax.max(p.y);
        }
    }
    if ymin >= ymax || !ymin.is_finite() || !ymax.is_finite() {
        ymin = -1.0;
        ymax = 1.0;
    }
    if (ymax - ymin).abs() < 1e-9 {
        ymin -= 1.0;
        ymax += 1.0;
    }
    let span = ymax - ymin;
    (ymin - span * 0.1, ymax + span * 0.1)
}

/// Map a canvas-local pixel to data coordinates.
///
/// Returns `None` when the pixel lies outside the plot area (padding or a
/// degenerate canvas). The mapping inverts the renderer's index-based layout
/// exactly: samples are uniform over `x_range`, so pixel → data is linear.
#[must_use]
pub fn graph_pixel_to_data(
    px: f32,
    py: f32,
    w: f32,
    h: f32,
    x_range: (f64, f64),
    y_range: (f64, f64),
) -> Option<(f64, f64)> {
    if w < 10.0 || h < 10.0 {
        return None;
    }
    let (from, to) = x_range;
    let (ymin, ymax) = y_range;
    // Ordered + finite ranges only. `partial_cmp` (instead of `>=`) keeps NaN
    // invalid: NaN compares as `None`, which is not `Less`.
    let x_ordered = from.partial_cmp(&to) == Some(std::cmp::Ordering::Less);
    let y_ordered = ymin.partial_cmp(&ymax) == Some(std::cmp::Ordering::Less);
    if !x_ordered || !y_ordered || !from.is_finite() || !to.is_finite() {
        return None;
    }
    let left = CANVAS_PAD;
    let right = w - CANVAS_PAD;
    let top = CANVAS_PAD;
    let bottom = h - CANVAS_PAD;
    if px < left || px > right || py < top || py > bottom {
        return None;
    }
    let span_x = right - left;
    let span_y = bottom - top;
    if span_x <= 0.0 || span_y <= 0.0 {
        return None;
    }
    let x = from + f64::from((px - left) / span_x) * (to - from);
    let y = ymin + f64::from((bottom - py) / span_y) * (ymax - ymin);
    Some((x, y))
}

/// One drawable series.
#[derive(Debug, Clone)]
pub struct GraphSeries {
    /// Samples (owned snapshot so `Program` stays `Clone`).
    pub samples: Vec<ocalc_engine::GraphPoint>,
    /// Curve color.
    pub color: Color,
}

/// Canvas program rendering all visible series + grid + axes.
///
/// `x_range`/`y_range` mirror the drawn mapping so hover events resolve to
/// the same data coordinates the curves show.
#[derive(Debug, Clone)]
pub struct GraphProgram {
    /// Series to draw.
    pub series: Vec<GraphSeries>,
    /// Draw background grid.
    pub show_grid: bool,
    /// Horizontal data range `(from, to)`.
    pub x_range: (f64, f64),
    /// Vertical data range `(ymin, ymax)` with margins.
    pub y_range: (f64, f64),
}

impl Program<Message> for GraphProgram {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Option<Action<Message>> {
        use iced::mouse;
        if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
            let px = position.x - bounds.x;
            let py = position.y - bounds.y;
            if let Some((x, y)) = graph_pixel_to_data(
                px,
                py,
                bounds.width,
                bounds.height,
                self.x_range,
                self.y_range,
            ) {
                return Some(Action::publish(Message::GraphHover(x, y)));
            }
        }
        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        if w < 10.0 || h < 10.0 {
            return vec![frame.into_geometry()];
        }
        let pad = CANVAS_PAD;
        let grid_color = Color::from_rgba(0.5, 0.5, 0.5, 0.25);
        let axis_color = Color::from_rgba(0.5, 0.5, 0.5, 0.7);

        // Data extents come from the shared helper so hover mapping and
        // rendering can never disagree.
        let (ymin, ymax) = self.y_range;
        let mut n = 0usize;
        for s in &self.series {
            n = n.max(s.samples.len());
        }

        let x_of = |i: usize| {
            if n <= 1 {
                pad
            } else {
                pad + (w - 2.0 * pad) * (i as f32 / (n - 1) as f32)
            }
        };
        let y_of = |y: f64| {
            let t = ((y - ymin) / (ymax - ymin)) as f32;
            h - pad - t * (h - 2.0 * pad)
        };

        // Grid (deciles) + axes.
        if self.show_grid {
            for k in 1..10 {
                let gx = pad + (w - 2.0 * pad) * (k as f32 / 10.0);
                frame.stroke(
                    &Path::line(Point::new(gx, pad), Point::new(gx, h - pad)),
                    Stroke::default().with_color(grid_color).with_width(1.0),
                );
                let gy = pad + (h - 2.0 * pad) * (k as f32 / 10.0);
                frame.stroke(
                    &Path::line(Point::new(pad, gy), Point::new(w - pad, gy)),
                    Stroke::default().with_color(grid_color).with_width(1.0),
                );
            }
        }
        let x_axis_y = y_of(0.0f64.clamp(ymin, ymax));
        frame.stroke(
            &Path::line(Point::new(pad, x_axis_y), Point::new(w - pad, x_axis_y)),
            Stroke::default().with_color(axis_color).with_width(1.5),
        );

        // One polyline per series; break at invalid points.
        for s in &self.series {
            let mut run: Vec<Point> = Vec::new();
            let flush = |frame: &mut Frame, run: &mut Vec<Point>| {
                if run.len() >= 2 {
                    let path = Path::new(|b| {
                        b.move_to(run[0]);
                        for p in &run[1..] {
                            b.line_to(*p);
                        }
                    });
                    frame.stroke(&path, Stroke::default().with_color(s.color).with_width(2.0));
                } else if run.len() == 1 {
                    let p = run[0];
                    frame.stroke(
                        &Path::line(p, Point::new(p.x + 0.5, p.y)),
                        Stroke::default().with_color(s.color).with_width(2.0),
                    );
                }
                run.clear();
            };
            for (i, p) in s.samples.iter().enumerate() {
                if !p.valid {
                    flush(&mut frame, &mut run);
                    continue;
                }
                run.push(Point::new(x_of(i), y_of(p.y)));
            }
            flush(&mut frame, &mut run);
        }

        vec![frame.into_geometry()]
    }
}

/// Graph mode: function list, range/zoom/pan/grid controls, canvas, status.
///
/// Phase 8: function list panel with per-function errors, zoom level
/// readout, pan/reset/grid controls, hover coordinate display, and calm
/// empty states when nothing is visible. Sampling still runs in
/// `commands::sample_graph_points` — this view only renders.
pub fn view(state: &AppState) -> Element<'_, Message> {
    use crate::views::{empty_state, heading};
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let muted = state.theme.colors.muted;
    let danger = state.theme.colors.danger;
    let g = &state.graph;

    // Function list panel: color dot + expr box + show/hide + remove.
    let mut funcs = column![heading(
        format!("Functions ({}/{})", g.funcs.len(), g.max_funcs),
        ty.body
    )]
    .spacing(sp.xs);
    for f in &g.funcs {
        let id = f.id;
        let color = ACCENT_PRESETS[f.color_idx % ACCENT_PRESETS.len()];
        let toggle_label = if f.visible { "Hide" } else { "Show" };
        let dot = if f.visible { "●" } else { "○" };
        funcs = funcs.push(
            row![
                text(dot).size(ty.body).color(color),
                text_input("y = f(x), e.g. sin(x)", &f.expr)
                    .on_input(move |s| Message::GraphFuncChanged(id, s))
                    .on_submit(Message::Nop)
                    .size(ty.small + 2.0)
                    .padding(sp.sm)
                    .width(Length::Fill),
                button(text(toggle_label).size(ty.small))
                    .on_press(Message::GraphToggleFunc(id))
                    .style(subtle_button()),
                button(text("✕").size(ty.small))
                    .on_press(Message::GraphRemoveFunc(id))
                    .style(crate::ods_theme::danger_button(danger)),
            ]
            .spacing(sp.sm)
            .align_y(Alignment::Center),
        );
        if let Some(err) = &f.error {
            let first = err.lines().next().unwrap_or("plot failed");
            funcs = funcs.push(text(format!("⚠ {first}")).size(ty.small).color(danger));
        }
    }
    let add_label = if g.funcs.len() >= g.max_funcs {
        format!("At most {} functions", g.max_funcs)
    } else {
        "+ Add function".to_owned()
    };
    funcs = funcs.push(
        button(text(add_label).size(ty.small))
            .on_press(Message::GraphAddFunc)
            .style(subtle_button()),
    );

    let from_box = text_input("from", &g.from_text)
        .on_input(|s| Message::GraphBoundChanged(0, s))
        .size(ty.small)
        .padding(sp.xs)
        .width(Length::Fixed(90.0));
    let to_box = text_input("to", &g.to_text)
        .on_input(|s| Message::GraphBoundChanged(1, s))
        .size(ty.small)
        .padding(sp.xs)
        .width(Length::Fixed(90.0));

    let pts_row = row![
        text(format!("Points: {}", g.points))
            .size(ty.small)
            .color(muted),
        button(text("101").size(ty.small)).on_press(Message::GraphPointsChanged(101)),
        button(text("501").size(ty.small)).on_press(Message::GraphPointsChanged(501)),
        button(text("1001").size(ty.small)).on_press(Message::GraphPointsChanged(1001)),
    ]
    .spacing(sp.xs)
    .align_y(Alignment::Center);

    // Zoom / pan / grid toolbar with live zoom readout.
    let zoom_label = if (g.zoom - 1.0).abs() < 0.01 {
        "Zoom 1×".to_owned()
    } else if g.zoom > 1.0 {
        format!("Zoom {:.1}×", g.zoom)
    } else {
        format!("Zoom 1/{:.0}×", 1.0 / g.zoom)
    };
    let nav_row = row![
        button(text("− Zoom").size(ty.small))
            .on_press(Message::GraphZoomOut)
            .style(subtle_button()),
        button(text("+ Zoom").size(ty.small))
            .on_press(Message::GraphZoomIn)
            .style(subtle_button()),
        button(text("◀ Pan").size(ty.small))
            .on_press(Message::GraphPanLeft)
            .style(subtle_button()),
        button(text("Pan ▶").size(ty.small))
            .on_press(Message::GraphPanRight)
            .style(subtle_button()),
        button(text("Reset").size(ty.small))
            .on_press(Message::GraphResetView)
            .style(subtle_button()),
        button(text(if g.show_grid { "☑ Grid" } else { "☐ Grid" }).size(ty.small))
            .on_press(Message::GraphToggleGrid)
            .style(subtle_button()),
        text(zoom_label).size(ty.small).color(muted),
    ]
    .spacing(sp.xs)
    .align_y(Alignment::Center);

    let (from, to) = g.effective_range();
    let status = {
        let total: usize = g.funcs.iter().map(|f| f.samples.len()).sum();
        let valid: usize = g
            .funcs
            .iter()
            .flat_map(|f| &f.samples)
            .filter(|p| p.valid)
            .count();
        let mut line = format!("range [{from:.3}, {to:.3}]  •  {total} points ({valid} valid)");
        // Hover readout from the canvas (cleared whenever data changes).
        match g.hover {
            Some((x, y)) => line.push_str(&format!("  •  cursor ({x:.3}, {y:.3})")),
            None => line.push_str("  •  hover the plot for coordinates"),
        }
        text(line).size(ty.small).color(muted)
    };

    let visible_count = g.funcs.iter().filter(|f| f.visible).count();
    let series: Vec<GraphSeries> = g
        .funcs
        .iter()
        .filter(|f| f.visible)
        .map(|f| GraphSeries {
            samples: f.samples.clone(),
            color: ACCENT_PRESETS[f.color_idx % ACCENT_PRESETS.len()],
        })
        .collect();
    let program = GraphProgram {
        x_range: (from, to),
        y_range: graph_y_extents(&series),
        series,
        show_grid: g.show_grid,
    };
    let canvas: Element<'_, Message> = Canvas::new(program)
        .width(Length::Fill)
        .height(Length::Fixed(320.0))
        .into();
    let plot: Element<'_, Message> = if visible_count == 0 {
        empty_state(
            "No visible functions — press Show on one above, or add a new one.",
            ty.small,
            muted,
        )
    } else {
        canvas
    };

    column![
        text("Graph y = f(x)").size(ty.heading),
        funcs,
        row![text("Range:").size(ty.small), from_box, to_box].spacing(sp.sm),
        pts_row,
        nav_row,
        plot,
        status,
        text("Tip: use x as the variable — e.g. sin(x), x^2, 1/x (gaps break cleanly).")
            .size(ty.tiny)
            .color(muted),
    ]
    .spacing(sp.sm)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
