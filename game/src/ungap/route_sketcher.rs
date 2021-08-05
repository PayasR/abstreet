use geom::{Circle, Distance, FindClosest};
use map_model::IntersectionID;
use widgetry::{
    Color, Drawable, EventCtx, GeomBatch, GfxCtx, HorizontalAlignment, Key, Line, Outcome, Panel,
    State, TextExt, VerticalAlignment, Widget,
};

use crate::app::{App, Transition};

pub struct RouteSketcher {
    top_panel: Panel,
    snap_to_intersections: FindClosest<IntersectionID>,
    route: Route,
    mode: Mode,
    preview: Drawable,
}

impl RouteSketcher {
    pub fn new_state(ctx: &mut EventCtx, app: &App) -> Box<dyn State<App>> {
        let mut snap_to_intersections = FindClosest::new(app.primary.map.get_bounds());
        for i in app.primary.map.all_intersections() {
            snap_to_intersections.add(i.id, i.polygon.points());
        }

        Box::new(RouteSketcher {
            top_panel: make_top_panel(ctx),
            snap_to_intersections,
            route: Route::new(),
            mode: Mode::Neutral,
            preview: Drawable::empty(ctx),
        })
    }

    fn mouseover_i(&self, ctx: &EventCtx) -> Option<IntersectionID> {
        let pt = ctx.canvas.get_cursor_in_map_space()?;
        self.snap_to_intersections
            .closest_pt(pt, Distance::meters(50.0))
            .map(|pair| pair.0)
    }

    fn update_mode(&mut self, ctx: &mut EventCtx, app: &App) {
        match self.mode {
            Mode::Neutral => {
                ctx.canvas_movement();
                if ctx.redo_mouseover() {
                    if let Some(i) = self.mouseover_i(ctx) {
                        self.mode = Mode::Hovering(i);
                    }
                }
            }
            Mode::Hovering(i) => {
                if ctx.normal_left_click() {
                    self.route.add_waypoint(app, i);
                    return;
                }

                if ctx.input.left_mouse_button_pressed() {
                    if let Some(idx) = self.route.idx(i) {
                        self.mode = Mode::Dragging { idx, at: i };
                        return;
                    }
                }

                if ctx.redo_mouseover() {
                    if let Some(i) = self.mouseover_i(ctx) {
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
                    if let Some(i) = self.mouseover_i(ctx) {
                        if i != at {
                            self.route.move_waypoint(idx, i);
                            self.mode = Mode::Dragging { idx, at: i };
                        }
                    }
                }
            }
        }
    }

    fn update_preview(&mut self, ctx: &mut EventCtx, app: &App) {
        let map = &app.primary.map;
        let mut batch = GeomBatch::new();

        // Draw the confirmed route
        for i in &self.route.waypoints {
            batch.push(
                Color::RED,
                Circle::new(map.get_i(*i).polygon.center(), Distance::meters(10.0)).to_polygon(),
            );
        }
        for pair in self.route.waypoints.windows(2) {
            // TODO unwrap after move_waypoint enforces validity
            if let Some((roads, _)) = map.simple_path_btwn(pair[0], pair[1]) {
                for r in roads {
                    batch.push(Color::RED.alpha(0.5), map.get_r(r).get_thick_polygon(map));
                }
            }
        }

        // Draw the current operation
        if let Mode::Hovering(i) = self.mode {
            batch.push(
                Color::BLUE,
                Circle::new(map.get_i(i).polygon.center(), Distance::meters(10.0)).to_polygon(),
            );
            if let Some(last) = self.route.waypoints.last() {
                if let Some((roads, _)) = map.simple_path_btwn(*last, i) {
                    for r in roads {
                        batch.push(Color::BLUE.alpha(0.5), map.get_r(r).get_thick_polygon(map));
                    }
                }
            }
        }
        if let Mode::Dragging { at, .. } = self.mode {
            batch.push(
                Color::BLUE,
                Circle::new(map.get_i(at).polygon.center(), Distance::meters(10.0)).to_polygon(),
            );
        }

        self.preview = batch.upload(ctx);
    }
}

impl State<App> for RouteSketcher {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut App) -> Transition {
        if let Outcome::Clicked(x) = self.top_panel.event(ctx) {
            match x.as_ref() {
                "Cancel" => {
                    return Transition::Pop;
                }
                _ => unreachable!(),
            }
        }

        let orig_route = self.route.clone();
        let orig_mode = self.mode.clone();
        self.update_mode(ctx, app);
        if self.route != orig_route || self.mode != orig_mode {
            self.update_preview(ctx, app);
        }

        Transition::Keep
    }

    fn draw(&self, g: &mut GfxCtx, _: &App) {
        self.top_panel.draw(g);
        g.redraw(&self.preview);
    }
}

fn make_top_panel(ctx: &mut EventCtx) -> Panel {
    Panel::new_builder(Widget::col(vec![
        Line("Draw a route").small_heading().into_widget(ctx),
        "Click to add a waypoint"
            .text_widget(ctx)
            .named("instructions"),
        "0 road segments selected".text_widget(ctx).named("count"),
        ctx.style()
            .btn_solid_destructive
            .text("Cancel")
            .hotkey(Key::Escape)
            .build_def(ctx),
    ]))
    .aligned(HorizontalAlignment::Center, VerticalAlignment::Top)
    .build(ctx)
}

#[derive(Clone, PartialEq)]
struct Route {
    waypoints: Vec<IntersectionID>,
}

impl Route {
    fn new() -> Route {
        Route {
            waypoints: Vec::new(),
        }
    }

    fn add_waypoint(&mut self, app: &App, i: IntersectionID) {
        if self.waypoints.is_empty() {
            self.waypoints.push(i);
            return;
        }

        if self.waypoints.contains(&i) {
            // Ignore. They can drag this point, though.
            return;
        }

        // Can we add this to the end of the path?
        let last = *self.waypoints.last().unwrap();
        if i != last {
            if app.primary.map.simple_path_btwn(last, i).is_some() {
                self.waypoints.push(i);
            }
        }
    }

    fn idx(&self, i: IntersectionID) -> Option<usize> {
        self.waypoints.iter().position(|x| *x == i)
    }

    fn move_waypoint(&mut self, idx: usize, new_i: IntersectionID) {
        // TODO Check validity with simple_path_btwn
        self.waypoints[idx] = new_i;
    }
}

#[derive(Clone, PartialEq)]
enum Mode {
    Neutral,
    Hovering(IntersectionID),
    Dragging { idx: usize, at: IntersectionID },
}
