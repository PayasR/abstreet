mod edit;
mod magnifying;
mod route_sketcher;

use geom::{Circle, Distance, Pt2D};
use map_gui::tools::{nice_map_name, CityPicker, PopupMsg};
use map_gui::ID;
use map_model::{LaneType, PathConstraints, Road};
use widgetry::{
    lctrl, Color, Drawable, EventCtx, Filler, GeomBatch, GfxCtx, HorizontalAlignment, Key, Line,
    Outcome, Panel, ScreenDims, State, Text, Toggle, VerticalAlignment, Widget,
};

use crate::app::{App, Transition};
use crate::common::Warping;

// #74B0FC
const PROTECTED_BIKE_LANE: Color = Color::rgb_f(0.455, 0.69, 0.988);
const PAINTED_BIKE_LANE: Color = Color::GREEN;
const GREENWAY: Color = Color::BLUE;

pub struct ExploreMap {
    top_panel: Panel,
    legend: Panel,
    magnifying_glass: Panel,
    unzoomed_layer: Drawable,
    elevation: bool,
}

impl ExploreMap {
    pub fn new_state(ctx: &mut EventCtx, app: &mut App) -> Box<dyn State<App>> {
        app.opts.show_building_driveways = false;

        Box::new(ExploreMap {
            top_panel: make_top_panel(ctx),
            legend: make_legend(ctx, app, false),
            magnifying_glass: make_magnifying_glass(ctx),
            unzoomed_layer: make_unzoomed_layer(ctx, app),
            elevation: false,
        })
    }
}

impl State<App> for ExploreMap {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut App) -> Transition {
        ctx.canvas_movement();

        if ctx.redo_mouseover() {
            let mut label = Text::new();
            if let Some(ID::Road(r)) = app.mouseover_unzoomed_roads_and_intersections(ctx) {
                let road = app.primary.map.get_r(r);
                label.add_line(Line(road.get_name(app.opts.language.as_ref())).small_heading());
                // TODO Indicate which direction is uphill
                label.add_line(Line(format!(
                    "{}% incline",
                    (road.percent_incline.abs() * 100.0).round()
                )));
            } else {
                // TODO Jittery panel
            }
            label.add_line(Line("Click for details").secondary());
            let label = label.into_widget(ctx);
            self.magnifying_glass.replace(ctx, "label", label);

            if self.elevation {
                let mut label = Text::new().into_widget(ctx);

                if ctx.canvas.cam_zoom < app.opts.min_zoom_for_detail {
                    if let Some(pt) = ctx.canvas.get_cursor_in_map_space() {
                        if let Some((elevation, _)) = app
                            .session
                            .elevation_contours
                            .value()
                            .unwrap()
                            .0
                            .closest_pt(pt, Distance::meters(300.0))
                        {
                            label = Line(format!("{} ft", elevation.to_feet().round()))
                                .into_widget(ctx);
                        }
                    }
                }
                self.legend.replace(ctx, "current elevation", label);
            }
        }

        if let Some(pt) = ctx.canvas.get_cursor_in_map_space() {
            if ctx.canvas.cam_zoom < app.opts.min_zoom_for_detail && ctx.normal_left_click() {
                return Transition::Push(Warping::new_state(
                    ctx,
                    pt,
                    Some(10.0),
                    None,
                    &mut app.primary,
                ));
            }
        }

        if let Outcome::Clicked(x) = self.top_panel.event(ctx) {
            match x.as_ref() {
                "about A/B Street" => {
                    return Transition::Push(PopupMsg::new_state(ctx, "TODO", vec!["TODO"]));
                }
                "Bike Master Plan" => {
                    return Transition::Push(PopupMsg::new_state(ctx, "TODO", vec!["TODO"]));
                }
                "Edit network" => {
                    return Transition::Push(edit::QuickEdit::new_state(ctx, app));
                }
                _ => unreachable!(),
            }
        }

        match self.legend.event(ctx) {
            Outcome::Clicked(x) => {
                return Transition::Push(match x.as_ref() {
                    "change map" => CityPicker::new_state(
                        ctx,
                        app,
                        Box::new(|ctx, app| {
                            Transition::Multi(vec![Transition::Pop, Transition::Replace(ExploreMap::new_state(ctx, app))])
                        }),
                    ),
                    // TODO Add physical picture examples
                    "highway" => PopupMsg::new_state(ctx, "Highways", vec!["Unless there's a separate trail (like on the 520 or I90 bridge), highways aren't accessible to biking"]),
                    "major street" => PopupMsg::new_state(ctx, "Major streets", vec!["Arterials have more traffic, but are often where businesses are located"]),
                    "minor street" => PopupMsg::new_state(ctx, "Minor streets", vec!["Local streets have a low volume of traffic and are usually comfortable for biking, even without dedicated infrastructure"]),
                    "trail" => PopupMsg::new_state(ctx, "Trails", vec!["Trails like the Burke Gilman are usually well-separated from vehicle traffic. The space is usually shared between people walking, cycling, and rolling."]),
                    "protected bike lane" => PopupMsg::new_state(ctx, "Protected bike lanes", vec!["Bike lanes separated from vehicle traffic by physical barriers or a few feet of striping"]),
                    "painted bike lane" => PopupMsg::new_state(ctx, "Painted bike lanes", vec!["Bike lanes without any separation from vehicle traffic. Often uncomfortably close to the \"door zone\" of parked cars."]),
                    "Stay Healthy Street / greenway" => PopupMsg::new_state(ctx, "Stay Healthy Streets and neighborhood greenways", vec!["Residential streets with additional signage and light barriers. These are intended to be low traffic, dedicated for people walking and biking."]),
                    // TODO Add URLs
                    "about the elevation data" => PopupMsg::new_state(ctx, "About the elevation data", vec!["Biking uphill next to traffic without any dedicated space isn't fun.", "Biking downhill next to traffic, especially in the door-zone of parked cars, and especially on Seattle's bumpy roads... is downright terrifying.", "", "Note the elevation data is incorrect near bridges.", "Thanks to King County LIDAR for the data, and Eldan Goldenberg for processing it."]),
                    _ => unreachable!(),
            });
            }
            Outcome::Changed(_) => {
                self.elevation = self.legend.is_checked("elevation");
                self.legend = make_legend(ctx, app, self.elevation);
                if self.elevation {
                    let name = app.primary.map.get_name().clone();
                    if app.session.elevation_contours.key() != Some(name.clone()) {
                        let mut low = Distance::ZERO;
                        let mut high = Distance::ZERO;
                        for i in app.primary.map.all_intersections() {
                            low = low.min(i.elevation);
                            high = high.max(i.elevation);
                        }
                        // TODO Maybe also draw the uphill arrows on the steepest streets?
                        let value =
                            crate::layer::elevation::make_elevation_contours(ctx, app, low, high);
                        app.session.elevation_contours.set(name, value);
                    }
                }
            }
            _ => {}
        }

        Transition::Keep
    }

    fn draw(&self, g: &mut GfxCtx, app: &App) {
        self.top_panel.draw(g);
        self.legend.draw(g);
        if g.canvas.cam_zoom < app.opts.min_zoom_for_detail {
            g.redraw(&self.unzoomed_layer);

            if self.elevation {
                if let Some((_, ref draw)) = app.session.elevation_contours.value() {
                    g.redraw(draw);
                }
            }

            self.magnifying_glass.draw(g);
            magnifying::draw(g, app, self.magnifying_glass.rect_of("glass"));
        }
    }
}

fn make_top_panel(ctx: &mut EventCtx) -> Panel {
    Panel::new_builder(Widget::row(vec![
        ctx.style()
            .btn_plain
            .btn()
            .image_path("system/assets/pregame/logo.svg")
            .image_dims(50.0)
            .build_widget(ctx, "about A/B Street"),
        // TODO Tab style?
        ctx.style()
            .btn_solid_primary
            .text("Today")
            .disabled(true)
            .build_def(ctx)
            .centered_vert(),
        ctx.style()
            .btn_solid_primary
            .text("Bike Master Plan")
            .build_def(ctx)
            .centered_vert(),
        ctx.style()
            .btn_solid_primary
            .icon_text("system/assets/tools/pencil.svg", "Edit network")
            .hotkey(lctrl(Key::E))
            .build_def(ctx)
            .centered_vert(),
    ]))
    .aligned(HorizontalAlignment::Center, VerticalAlignment::Top)
    .build(ctx)
}

fn make_legend(ctx: &mut EventCtx, app: &App, elevation: bool) -> Panel {
    Panel::new_builder(Widget::col(vec![
        Widget::custom_row(vec![
            Line("Bike Network")
                .small_heading()
                .into_widget(ctx)
                .margin_right(18),
            ctx.style()
                .btn_popup_icon_text(
                    "system/assets/tools/map.svg",
                    nice_map_name(app.primary.map.get_name()),
                )
                .hotkey(lctrl(Key::L))
                .build_widget(ctx, "change map")
                .margin_right(8),
        ]),
        // TODO Looks too close to access restrictions
        legend(ctx, app.cs.unzoomed_highway, "highway"),
        legend(ctx, app.cs.unzoomed_arterial, "major street"),
        legend(ctx, app.cs.unzoomed_residential, "minor street"),
        legend(ctx, app.cs.unzoomed_trail, "trail"),
        legend(ctx, PROTECTED_BIKE_LANE, "protected bike lane"),
        legend(ctx, PAINTED_BIKE_LANE, "painted bike lane"),
        legend(ctx, GREENWAY, "Stay Healthy Street / greenway"),
        // TODO Distinguish door-zone bike lanes?
        // TODO Call out bike turning boxes?
        // TODO Call out bike signals?
        Widget::row(vec![
            Toggle::checkbox(ctx, "elevation", Key::E, elevation),
            ctx.style()
                .btn_plain
                .icon("system/assets/tools/info.svg")
                .build_widget(ctx, "about the elevation data")
                .centered_vert(),
            Text::from(Line("Click for details").secondary())
                .into_widget(ctx)
                .named("current elevation")
                .centered_vert(),
        ]),
    ]))
    .aligned(HorizontalAlignment::Right, VerticalAlignment::Bottom)
    .build(ctx)
}

fn legend(ctx: &mut EventCtx, color: Color, label: &str) -> Widget {
    let radius = 15.0;
    Widget::row(vec![
        GeomBatch::from(vec![(
            color,
            Circle::new(Pt2D::new(radius, radius), Distance::meters(radius)).to_polygon(),
        )])
        .into_widget(ctx)
        .centered_vert(),
        ctx.style()
            .btn_plain
            .text(label)
            .build_def(ctx)
            .centered_vert(),
    ])
}

fn make_magnifying_glass(ctx: &mut EventCtx) -> Panel {
    Panel::new_builder(
        Widget::col(vec![
            Filler::fixed_dims(ScreenDims::new(300.0, 300.0)).named("glass"),
            Text::new().into_widget(ctx).named("label"),
        ])
        .padding(16)
        .outline((4.0, Color::BLACK)),
    )
    .aligned(HorizontalAlignment::LeftInset, VerticalAlignment::TopInset)
    .build(ctx)
}

pub fn make_unzoomed_layer(ctx: &mut EventCtx, app: &App) -> Drawable {
    let mut batch = GeomBatch::new();
    for r in app.primary.map.all_roads() {
        if r.is_cycleway() {
            continue;
        }

        if is_greenway(r) {
            // TODO This color is going to look hilarious on top of the already bizarre pink
            batch.push(GREENWAY.alpha(0.8), r.get_thick_polygon(&app.primary.map));
        }

        // Don't cover up the arterial/local classification -- add thick side lines to show bike
        // facilties in each direction.
        let mut bike_lane_left = false;
        let mut buffer_left = false;
        let mut bike_lane_right = false;
        let mut buffer_right = false;
        let mut on_left = true;
        for (_, _, lt) in r.lanes_ltr() {
            if lt == LaneType::Driving || lt == LaneType::Bus {
                // We're walking the lanes from left-to-right. So as soon as we hit a vehicle lane,
                // any bike lane we find is on the right side of the road.
                // (Barring really bizarre things like a bike lane in the middle of the road)
                on_left = false;
            } else if lt == LaneType::Biking {
                if on_left {
                    bike_lane_left = true;
                } else {
                    bike_lane_right = true;
                }
            } else if matches!(lt, LaneType::Buffer(_)) {
                if on_left {
                    buffer_left = true;
                } else {
                    buffer_right = true;
                }
            }

            let half_width = r.get_half_width(&app.primary.map);
            for (shift, bike_lane, buffer) in [
                (-1.0, bike_lane_left, buffer_left),
                (1.0, bike_lane_right, buffer_right),
            ] {
                let color = if bike_lane && buffer {
                    PROTECTED_BIKE_LANE
                } else if bike_lane {
                    PAINTED_BIKE_LANE
                } else {
                    // If we happen to have a buffer, but no bike lane, let's just not ask
                    // questions...
                    continue;
                };
                if let Ok(pl) = r.center_pts.shift_either_direction(shift * half_width) {
                    batch.push(color, pl.make_polygons(0.5 * half_width));
                }
            }
        }
    }
    batch.upload(ctx)
}

// TODO Check how other greenways are tagged.
// https://www.openstreetmap.org/way/262778812 has bicycle=designated, cycleway=shared_lane...
fn is_greenway(road: &Road) -> bool {
    !road
        .access_restrictions
        .allow_through_traffic
        .contains(PathConstraints::Car)
        && road
            .access_restrictions
            .allow_through_traffic
            .contains(PathConstraints::Bike)
}
