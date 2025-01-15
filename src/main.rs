#![feature(linked_list_cursors, linked_list_retain)]
use dragon::{CurveFlags, DragonCurve};
use eframe::egui;
use eframe::egui::epaint::PathStroke;
use std::cmp::Ordering;

mod dragon;

trait MakeStroke {
    fn stroke(&mut self) -> PathStroke;
}
impl MakeStroke for (f32, egui::Color32) {
    fn stroke(&mut self) -> PathStroke {
        (*self).into()
    }
}
impl MakeStroke for egui::Stroke {
    fn stroke(&mut self) -> PathStroke {
        (*self).into()
    }
}

struct GradientStroke {
    width: f32,
    count: usize,
    max: usize,
    grad: colorous::Gradient,
}
impl MakeStroke for GradientStroke {
    fn stroke(&mut self) -> PathStroke {
        let (r, g, b) = self
            .grad
            .eval_rational(std::cmp::min(self.count, self.max), self.max)
            .into_tuple();
        if self.count < self.max {
            self.count += 1;
        }
        (self.width, egui::Color32::from_rgb(r, g, b)).into()
    }
}

struct GradientBands<'a> {
    width: f32,
    count: usize,
    max: usize,
    colors: &'a [egui::Color32],
}
impl MakeStroke for GradientBands<'_> {
    fn stroke(&mut self) -> PathStroke {
        let ratio = (self.count * (self.colors.len() - 1)) as f32 / self.max as f32;
        let idx = ratio as usize;
        let frac = ratio.fract();
        let color = if frac < f32::EPSILON {
            self.colors[idx]
        } else {
            self.colors[idx].lerp_to_gamma(self.colors[idx + 1], frac)
        };
        if self.count < self.max - 1 {
            self.count += 1;
        }
        (self.width, color).into()
    }
}
struct SolidBands<'a> {
    width: f32,
    count: usize,
    max: usize,
    colors: &'a [egui::Color32],
}
impl MakeStroke for SolidBands<'_> {
    fn stroke(&mut self) -> PathStroke {
        let idx = (self.count * self.colors.len()) / self.max;
        if self.count < self.max - 1 {
            self.count += 1;
        }
        (self.width, self.colors[idx]).into()
    }
}
struct EguiDraw<'a, S> {
    painter: &'a egui::Painter,
    pos: egui::Pos2,
    stroke: S,
}
impl<S: MakeStroke> dragon::Draw for EguiDraw<'_, S> {
    type Output = ();

    fn line(&mut self, x: f32, y: f32) {
        let old = self.pos;
        self.pos += egui::vec2(x, y);
        self.painter
            .line_segment([old, self.pos], self.stroke.stroke());
    }
    fn horiz(&mut self, x: f32) {
        let old = self.pos.x;
        self.pos.x += x;
        self.painter
            .hline(old..=self.pos.x, self.pos.y, self.stroke.stroke());
    }
    fn vert(&mut self, y: f32) {
        let old = self.pos.y;
        self.pos.y += y;
        self.painter
            .vline(self.pos.x, old..=self.pos.y, self.stroke.stroke());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GradientKind {
    Viridis,
    Plasma,
    Warm,
    Cool,
    Sinebow,
}
impl GradientKind {
    fn into_colorous(&self) -> colorous::Gradient {
        match self {
            Self::Viridis => colorous::VIRIDIS,
            Self::Plasma => colorous::PLASMA,
            Self::Warm => colorous::WARM,
            Self::Cool => colorous::COOL,
            Self::Sinebow => colorous::SINEBOW,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PrideFlag {
    Rainbow,
    Trans,
}
impl PrideFlag {
    fn into_bands(&self) -> &'static [egui::Color32] {
        match self {
            Self::Rainbow => RAINBOW_FLAG,
            Self::Trans => TRANS_FLAG,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Coloring {
    None,
    Colorous,
    SolidPride,
    GradientPride,
}

const RAINBOW_FLAG: &[egui::Color32] = &[
    egui::Color32::from_rgb(0xe5, 0x00, 0x00),
    egui::Color32::from_rgb(0xfe, 0x8d, 0x00),
    egui::Color32::from_rgb(0xff, 0xee, 0x00),
    egui::Color32::from_rgb(0x02, 0x81, 0x21),
    egui::Color32::from_rgb(0x00, 0x4c, 0xff),
    egui::Color32::from_rgb(0x76, 0x00, 0x88),
];
const TRANS_FLAG: &[egui::Color32] = &[
    egui::Color32::from_rgb(0x5b, 0xcf, 0xfb),
    egui::Color32::from_rgb(0xf5, 0xab, 0xb9),
    egui::Color32::from_rgb(0xff, 0xff, 0xff),
    egui::Color32::from_rgb(0xf5, 0xab, 0xb9),
    egui::Color32::from_rgb(0x5b, 0xcf, 0xfb),
];

fn main() {
    let start = dragon::Dir::Np0;
    let mut show = true;
    let mut flip = false;
    let mut levy = false;
    let mut depth = 0;
    let mut curve = DragonCurve::new(start, CurveFlags::DRAGON);
    let mut coloring = Coloring::None;
    let mut gradient = GradientKind::Viridis;
    let mut pride_flag = PrideFlag::Rainbow;
    let res = eframe::run_simple_native("Dragon", Default::default(), move |ctx, _| {
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            show = !show;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let rect = ui.max_rect();
                let size = rect.size().min_elem();
                let offset = match rect.aspect_ratio().partial_cmp(&1.0) {
                    Some(Ordering::Less) => egui::vec2(0.0, (rect.height() - size) * 0.5),
                    Some(Ordering::Greater) => egui::vec2((rect.width() - size) * 0.5, 0.0),
                    _ => egui::Vec2::ZERO,
                };
                let mut step = size / (1 << (depth / 2) + 1) as f32;
                if depth & 1 != 0 {
                    step *= std::f32::consts::FRAC_1_SQRT_2;
                }
                let pos = rect.min + offset + egui::vec2(size * 0.25, size * 0.5);
                match coloring {
                    Coloring::None => {
                        let mut draw = EguiDraw {
                            painter: ui.painter(),
                            pos,
                            stroke: ui.style().visuals.widgets.active.fg_stroke,
                        };
                        for seg in curve.list() {
                            seg.draw(&mut draw, step);
                        }
                    }
                    Coloring::Colorous => {
                        let mut draw = EguiDraw {
                            painter: ui.painter(),
                            pos,
                            stroke: GradientStroke {
                                width: ui.style().visuals.widgets.active.fg_stroke.width,
                                count: 0,
                                max: curve.len(),
                                grad: gradient.into_colorous(),
                            },
                        };
                        for seg in curve.list() {
                            seg.draw(&mut draw, step);
                        }
                    }
                    Coloring::SolidPride => {
                        let mut draw = EguiDraw {
                            painter: ui.painter(),
                            pos,
                            stroke: SolidBands {
                                width: ui.style().visuals.widgets.active.fg_stroke.width,
                                count: 0,
                                max: curve.len(),
                                colors: pride_flag.into_bands(),
                            },
                        };
                        for seg in curve.list() {
                            seg.draw(&mut draw, step);
                        }
                    }
                    Coloring::GradientPride => {
                        let mut draw = EguiDraw {
                            painter: ui.painter(),
                            pos,
                            stroke: GradientBands {
                                width: ui.style().visuals.widgets.active.fg_stroke.width,
                                count: 0,
                                max: curve.len(),
                                colors: pride_flag.into_bands(),
                            },
                        };
                        for seg in curve.list() {
                            seg.draw(&mut draw, step);
                        }
                    }
                }
            });
        });
        if show {
            egui::Window::new("Curve Options").show(ctx, |ui| {
                let mut changed = false;
                changed |= ui.checkbox(&mut flip, "Flip").changed();
                changed |= ui.checkbox(&mut levy, "Levy").changed();
                ui.add(egui::Slider::new(&mut depth, 0..=16)).changed();
                if changed {
                    let mut flags = CurveFlags::NONE;
                    if flip {
                        flags |= CurveFlags::FLIP;
                    }
                    if levy {
                        flags |= CurveFlags::LEVY;
                    }
                    curve = DragonCurve::new(start, flags);
                }
                curve.set_depth(depth);
            });
            egui::Window::new("Display Options").show(ctx, |ui| {
                egui::ComboBox::new("Coloring", "Coloring")
                    .selected_text(format!("{coloring:?}"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut coloring, Coloring::None, "None");
                        ui.selectable_value(&mut coloring, Coloring::Colorous, "Colorous");
                        ui.selectable_value(&mut coloring, Coloring::SolidPride, "SolidPride");
                        ui.selectable_value(&mut coloring, Coloring::GradientPride, "GradientPride");
                    });
                match coloring {
                    Coloring::Colorous => {
                        egui::ComboBox::new("Gradient", "Gradient")
                            .selected_text(format!("{gradient:?}"))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut gradient,
                                    GradientKind::Viridis,
                                    "Viridis",
                                );
                                ui.selectable_value(&mut gradient, GradientKind::Plasma, "Plasma");
                                ui.selectable_value(&mut gradient, GradientKind::Warm, "Warm");
                                ui.selectable_value(&mut gradient, GradientKind::Cool, "Cool");
                                ui.selectable_value(
                                    &mut gradient,
                                    GradientKind::Sinebow,
                                    "Sinebow",
                                );
                            });
                    }
                    Coloring::SolidPride | Coloring::GradientPride => {
                        egui::ComboBox::new("Flag", "Flag")
                            .selected_text(format!("{pride_flag:?}"))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut pride_flag, PrideFlag::Rainbow, "Rainbow");
                                ui.selectable_value(&mut pride_flag, PrideFlag::Trans, "Trans");
                            });
                    }
                    _ => {}
                }
            });
        }
    });
    if let Err(err) = res {
        eprintln!("Failed to run app: {err}");
    }
}
