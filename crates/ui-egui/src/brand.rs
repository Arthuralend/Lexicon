//! Original Lexicon document/orbit mark, drawn at the interface's resolution.
use egui::{Color32, Painter, Rect, Stroke, pos2};

pub fn paint(painter: &Painter, rect: Rect, ink: Color32) {
    let at = |x: f32, y: f32| pos2(rect.left() + x * rect.width(), rect.top() + y * rect.height());
    let stroke = Stroke::new((rect.width() / 22.0).max(1.0), ink);
    let line = |points: &[(f32, f32)]| {
        painter.add(egui::Shape::line(points.iter().map(|&(x, y)| at(x, y)).collect(), stroke));
    };
    line(&[(0.35, 0.25), (0.56, 0.25), (0.68, 0.37), (0.68, 0.72), (0.35, 0.72), (0.35, 0.25)]);
    line(&[(0.56, 0.25), (0.56, 0.37), (0.68, 0.37)]);
    line(&[(0.45, 0.40), (0.45, 0.60), (0.58, 0.60)]);
    for (start, end) in [(0.1_f32, 2.8_f32), (3.3, 5.8)] {
        let points = (0..32)
            .map(|i| {
                let angle = start + (end - start) * i as f32 / 31.0;
                let x = 0.44 * angle.cos();
                let y = 0.25 * angle.sin();
                at(0.5 + x * 0.90 - y * 0.44, 0.5 + x * 0.44 + y * 0.90)
            })
            .collect();
        painter.add(egui::Shape::line(points, stroke));
    }
    for (x, y) in [(0.28, 0.82), (0.72, 0.18)] {
        painter.circle_stroke(at(x, y), rect.width() * 0.055, stroke);
    }
}
