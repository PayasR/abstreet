use geom::{Distance, FindClosest};
use map_model::{IntersectionID, RoadID};
use widgetry::{
    Color, Drawable, EventCtx, GeomBatch, GfxCtx, HorizontalAlignment, Key, Line, Outcome, Panel,
    RewriteColor, State, TextExt, Toggle, VerticalAlignment, Widget,
};

use crate::app::{App, Transition};

// A new attempt at RoadSelector.
pub struct DraggyRoute {
    top_panel: Panel,
    snap_to_intersections: FindClosest<IntersectionID>,
    i1: Option<IntersectionID>,
    hovering: Option<IntersectionID>,
    // TODO Cache based on i1 and i2? (path, draw)
    preview: (Vec<RoadID>, Drawable),
}

impl DraggyRoute {
    pub fn new_state(ctx: &mut EventCtx, app: &mut App) -> Box<dyn State<App>> {
        let mut snap_to_intersections = FindClosest::new(app.primary.map.get_bounds());
        for i in app.primary.map.all_intersections() {
            snap_to_intersections.add(i.id, i.polygon.points());
        }

        Box::new(DraggyRoute {
            top_panel: make_top_panel(ctx),
            snap_to_intersections,
            i1: None,
            hovering: None,
            preview: (Vec::new(), Drawable::empty(ctx)),
        })
    }
}

impl State<App> for DraggyRoute {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut App) -> Transition {
        ctx.canvas_movement();

        if ctx.redo_mouseover() {
            let new_hovering = ctx.canvas.get_cursor_in_map_space().and_then(|pt| {
                self.snap_to_intersections
                    .closest_pt(pt, Distance::meters(50.0))
                    .map(|pair| pair.0)
            });
            if new_hovering != self.hovering {
                self.preview = make_preview(ctx, app, new_hovering, self.i1);
                self.hovering = new_hovering;
            }

            // TODO Drag the first marker, or the second once it's placed.
        }
        if self.i1.is_none() && self.hovering.is_some() && ctx.normal_left_click() {
            self.i1 = self.hovering;
            self.preview = make_preview(ctx, app, self.hovering, self.i1);
        }
        // TODO Confirm path

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
        g.redraw(&self.preview.1);
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

fn make_preview(
    ctx: &mut EventCtx,
    app: &App,
    hovering: Option<IntersectionID>,
    i1: Option<IntersectionID>,
) -> (Vec<RoadID>, Drawable) {
    let mut batch = GeomBatch::new();
    let mut path = Vec::new();
    let map = &app.primary.map;

    if let Some(i1) = i1 {
        batch.append(
            GeomBatch::load_svg(ctx, "system/assets/timeline/start_pos.svg")
                .centered_on(map.get_i(i1).polygon.center()),
        );

        if let Some(i2) = hovering {
            if i1 != i2 {
                batch.append(
                    GeomBatch::load_svg(ctx, "system/assets/timeline/goal_pos.svg")
                        .centered_on(map.get_i(i2).polygon.center())
                        .color(RewriteColor::ChangeAlpha(0.8)),
                );

                if let Some(roads) = map.simple_path_btwn(i1, i2) {
                    for r in roads {
                        path.push(r);
                        batch.push(Color::RED.alpha(0.8), map.get_r(r).get_thick_polygon(map));
                    }
                }
            }
        }
    } else if let Some(i1) = hovering {
        batch.append(
            GeomBatch::load_svg(ctx, "system/assets/timeline/start_pos.svg")
                .centered_on(map.get_i(i1).polygon.center())
                .color(RewriteColor::ChangeAlpha(0.8)),
        );
    }

    (path, batch.upload(ctx))
}
