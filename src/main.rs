#![feature(linked_list_cursors, linked_list_retain)]
use eframe::egui;
use eframe::egui::epaint::PathStroke;
use dragon::{DragonCurve, CurveFlags};
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
        self.painter.line_segment([old, self.pos], self.stroke.stroke());
    }
    fn horiz(&mut self, x: f32) {
        let old = self.pos.x;
        self.pos.x += x;
        self.painter.hline(old..=self.pos.x, self.pos.y, self.stroke.stroke());
    }
    fn vert(&mut self, y: f32) {
        let old = self.pos.y;
        self.pos.y += y;
        self.painter.vline(self.pos.x, old..=self.pos.y, self.stroke.stroke());
    }
}


fn main() {
    let mut flip = false;
    let mut levy = false;
    let mut start = dragon::Dir::Np0;
    let mut depth = 0;
    let mut curve = DragonCurve::new(start, CurveFlags::DRAGON);
    let res = eframe::run_simple_native("Dragon", Default::default(), move |ctx, _| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let rect = ui.max_rect();
                let size = rect.size().min_elem();
                let offset = match rect.aspect_ratio().partial_cmp(&1.0) {
                    Some(Ordering::Less) => {
                        egui::vec2(0.0, (rect.height() - size) * 0.5)
                    }
                    Some(Ordering::Greater) => {
                        egui::vec2((rect.width() - size) * 0.5, 0.0)
                    },
                    _ => egui::Vec2::ZERO,
                };
                let mut draw = EguiDraw {
                    painter: ui.painter(),
                    pos: rect.min + offset + egui::vec2(size * 0.25, size * 0.5),
                    stroke: ui.style().visuals.widgets.active.fg_stroke,
                };
                let mut step = size / (1 << (depth / 2) + 1) as f32;
                if depth & 1 != 0 {
                    step *= std::f32::consts::FRAC_1_SQRT_2;
                }
                for seg in curve.list() {
                    seg.draw(&mut draw, step);
                }
            });
        });
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
            
        });
    });
    if let Err(err) = res {
        eprintln!("Failed to run app: {err}");
    }
}
