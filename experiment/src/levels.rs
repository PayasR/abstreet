use abstutil::MapName;
use map_gui::tools::{open_browser, PopupMsg};
use map_gui::SimpleApp;
use map_model::osm;
use widgetry::{
    Btn, DrawBaselayer, EventCtx, GfxCtx, Key, Line, Outcome, Panel, State, Text, Transition,
    Widget,
};

#[derive(Clone)]
pub struct Level {
    pub title: &'static str,
    pub map: MapName,
    pub start: osm::NodeID,
    pub minimap_zoom: usize,
    pub num_upzones: usize,
    pub vehicles: Vec<&'static str>,
}

// TODO Like Challenge::all; cache with lazy static?
fn all_levels() -> Vec<Level> {
    vec![
        Level {
            title: "Level 1 - a small neighborhood",
            map: MapName::seattle("montlake"),
            start: osm::NodeID(53084814),
            minimap_zoom: 1,
            num_upzones: 0,
            vehicles: vec!["sleigh"],
        },
        Level {
            title: "Level 2 - a small neighborhood with upzones",
            map: MapName::seattle("montlake"),
            start: osm::NodeID(53084814),
            minimap_zoom: 1,
            num_upzones: 3,
            vehicles: vec!["bike", "cargo bike", "sleigh"],
        },
        Level {
            title: "Level 3 - Magnolia",
            map: MapName::seattle("ballard"),
            start: osm::NodeID(53117102),
            minimap_zoom: 2,
            num_upzones: 5,
            vehicles: vec!["bike", "cargo bike", "sleigh"],
        },
    ]
}

pub struct TitleScreen {
    panel: Panel,
}

impl TitleScreen {
    pub fn new(ctx: &mut EventCtx) -> Box<dyn State<SimpleApp>> {
        let levels = all_levels();

        Box::new(TitleScreen {
            panel: Panel::new(
                Widget::col(vec![
                    Btn::svg_def("system/assets/pregame/quit.svg")
                        .build(ctx, "quit", Key::Escape)
                        .align_left(),
                    {
                        let mut txt = Text::from(Line("15 minute Santa").display_title());
                        txt.add(Line("Created by Dustin Carlino, Yuwen Li, & Michael Kirk"));
                        txt.draw(ctx).centered_horiz()
                    },
                    Btn::text_bg1("Santa character created by @parallaxcreativedesign").build(
                        ctx,
                        "open https://www.instagram.com/parallaxcreativedesign/",
                        None,
                    ),
                    Btn::text_bg2("Instructions").build_def(ctx, None),
                    Widget::row(
                        levels
                            .into_iter()
                            .map(|lvl| Btn::text_bg2(lvl.title).build_def(ctx, None))
                            .collect(),
                    ),
                ])
                .evenly_spaced(),
            )
            .exact_size_percent(90, 85)
            .build_custom(ctx),
        })
    }
}

impl State<SimpleApp> for TitleScreen {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut SimpleApp) -> Transition<SimpleApp> {
        match self.panel.event(ctx) {
            Outcome::Clicked(x) => match x.as_ref() {
                "quit" => {
                    std::process::exit(0);
                }
                "Instructions" => {
                    // TODO As I'm writing the range argument, I don't buy the hybrid motor.
                    // Wireless Tesla energy instead?
                    return Transition::Push(PopupMsg::new(
                        ctx,
                        "Instructions",
                        vec![
                            "It's Christmas Eve, so it's time for Santa to deliver presents in \
                             Seattle. 2020 has thoroughly squashed any remaining magic out of the \
                             world, so your sleigh can only hold so many presents at a time.",
                            "Deliver presents as fast as you can. When you run out, refill from a \
                             yellow-colored store.",
                            "It's faster to deliver to buildings with multiple families inside.",
                            "",
                            "When you deliver enough presents, a little bit of magic is restored, \
                             and you can upzone buildings to make your job easier.",
                            "If you're having trouble delivering to houses far away from \
                             businesses, why not build a new grocery store where it might be \
                             needed?",
                        ],
                    ));
                }
                x => {
                    if let Some(url) = x.strip_prefix("open ") {
                        open_browser(url.to_string());
                        return Transition::Keep;
                    }

                    for lvl in all_levels() {
                        if x == lvl.title {
                            return Transition::Push(crate::upzone::Picker::new(ctx, app, lvl));
                        }
                    }
                    panic!("Unknown action {}", x);
                }
            },
            _ => {}
        }

        Transition::Keep
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::Custom
    }

    fn draw(&self, g: &mut GfxCtx, app: &SimpleApp) {
        g.clear(app.cs.dialog_bg);
        self.panel.draw(g);
    }
}
