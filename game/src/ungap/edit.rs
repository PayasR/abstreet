use geom::{Circle, Distance, FindClosest};
use map_model::{IntersectionID, RoadID};
use widgetry::{
    Color, Drawable, EventCtx, GeomBatch, GfxCtx, HorizontalAlignment, Key, Line, Outcome, Panel,
    State, TextExt, Toggle, VerticalAlignment, Widget,
};

use crate::app::{App, Transition};

// A new attempt at RoadSelector.
pub struct SketchRoute {
    top_panel: Panel,
    snap_to_intersections: FindClosest<IntersectionID>,
    // TODO Explicit waypoints and implicit stuff in between.
    path: Vec<IntersectionID>,
    mode: Mode,
    preview: Drawable,
}

#[derive(Clone, PartialEq)]
enum Mode {
    Neutral,
    Hovering(IntersectionID),
    Dragging { idx: usize, at: IntersectionID },
}

impl SketchRoute {
    pub fn new_state(ctx: &mut EventCtx, app: &mut App) -> Box<dyn State<App>> {
        let mut snap_to_intersections = FindClosest::new(app.primary.map.get_bounds());
        for i in app.primary.map.all_intersections() {
            snap_to_intersections.add(i.id, i.polygon.points());
        }

        Box::new(SketchRoute {
            top_panel: make_top_panel(ctx),
            snap_to_intersections,
            path: Vec::new(),
            mode: Mode::Neutral,
            preview: Drawable::empty(ctx),
        })
    }

    fn update_mode(&mut self, ctx: &mut EventCtx, app: &App) {
        let map = &app.primary.map;

        match self.mode {
            Mode::Neutral => {
                ctx.canvas_movement();
                if ctx.redo_mouseover() {
                    if let Some(i) = ctx.canvas.get_cursor_in_map_space().and_then(|pt| {
                        self.snap_to_intersections
                            .closest_pt(pt, Distance::meters(50.0))
                            .map(|pair| pair.0)
                    }) {
                        self.mode = Mode::Hovering(i);
                    }
                }
            }
            Mode::Hovering(i) => {
                if ctx.normal_left_click() {
                    if self.path.is_empty() {
                        self.path.push(i);
                    } else if self.path.contains(&i) {
                        // Ignore. They can drag this point, though.
                    } else if let Some(new_path) =
                        map.simple_path_btwn(*self.path.last().unwrap(), i)
                    {
                        self.path.pop();
                        // TODO This will mess up the path for sure
                        for r in new_path {
                            let r = map.get_r(r);
                            self.path.push(r.src_i);
                            self.path.push(r.dst_i);
                        }
                    }
                    return;
                }

                if ctx.input.left_mouse_button_pressed() {
                    if let Some(idx) = self.path.iter().position(|x| *x == i) {
                        self.mode = Mode::Dragging { idx, at: i };
                        return;
                    }
                }

                if ctx.redo_mouseover() {
                    if let Some(i) = ctx.canvas.get_cursor_in_map_space().and_then(|pt| {
                        self.snap_to_intersections
                            .closest_pt(pt, Distance::meters(50.0))
                            .map(|pair| pair.0)
                    }) {
                        self.mode = Mode::Hovering(i);
                    } else {
                        self.mode = Mode::Neutral;
                    }
                }
            }
            Mode::Dragging { idx, at } => {
                if ctx.input.left_mouse_button_released() {
                    self.mode = Mode::Hovering(at);
                    return;
                }

                if ctx.redo_mouseover() {
                    if let Some(i) = ctx.canvas.get_cursor_in_map_space().and_then(|pt| {
                        self.snap_to_intersections
                            .closest_pt(pt, Distance::meters(50.0))
                            .map(|pair| pair.0)
                    }) {
                        if i != at {
                            // Modify the path!
                        }
                    }
                }
            }
        }
    }

    fn update_preview(&mut self, ctx: &mut EventCtx, app: &App) {
        let mut batch = GeomBatch::new();
        let map = &app.primary.map;

        for i in &self.path {
            batch.push(
                Color::RED,
                Circle::new(map.get_i(*i).polygon.center(), Distance::meters(10.0))
                    .to_outline(Distance::meters(2.0))
                    .unwrap(),
            );
        }

        // TODO Mode

        self.preview = batch.upload(ctx);
    }
}

impl State<App> for SketchRoute {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut App) -> Transition {
        let orig_path = self.path.clone();
        let orig_mode = self.mode.clone();
        self.update_mode(ctx, app);
        if self.path != orig_path || self.mode != orig_mode {
            self.update_preview(ctx, app);
        }

        if let Outcome::Clicked(x) = self.top_panel.event(ctx) {
            match x.as_ref() {
                "Cancel" => {
                    return Transition::Pop;
                }
                _ => unreachable!(),
            }
        }

        Transition::Keep
    }

    fn draw(&self, g: &mut GfxCtx, _: &App) {
        // TODO Still draw the layer
        self.top_panel.draw(g);
        g.redraw(&self.preview);
    }
}

fn make_top_panel(ctx: &mut EventCtx) -> Panel {
    Panel::new_builder(Widget::col(vec![
        Line("Add bike lanes to a route")
            .small_heading()
            .into_widget(ctx),
        "Click two intersections".text_widget(ctx),
        Toggle::checkbox(
            ctx,
            "protect bike lanes with a buffer (if not, just paint)",
            None,
            false,
        ),
        // TODO Show elevation profile of the route
        // TODO left, right, both sides?
        ctx.style()
            .btn_plain_destructive
            .text("Cancel")
            .hotkey(Key::Escape)
            .build_def(ctx),
    ]))
    .aligned(HorizontalAlignment::Center, VerticalAlignment::Top)
    .build(ctx)
}
