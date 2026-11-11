//! Iconos vectoriales de línea fina, dibujados con código (sin imágenes).

use eframe::egui::epaint::PathShape;
use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Rounding, Stroke};

use super::glass::{falloff, luminance, radial, unit};
use crate::domain::Category;

const WEIGHT: f32 = 1.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Compose,
    List,
    Chart,
    Alert,
    Check,
    Info,
}

/// Lápiz en coordenadas relativas al tamaño del icono (-0.5‥0.5).
struct Pen<'a> {
    painter: &'a Painter,
    center: Pos2,
    size: f32,
    stroke: Stroke,
}

impl<'a> Pen<'a> {
    fn new(painter: &'a Painter, center: Pos2, size: f32, color: Color32) -> Self {
        Self { painter, center, size, stroke: Stroke::new(WEIGHT, color) }
    }

    fn at(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.center.x + x * self.size, self.center.y + y * self.size)
    }

    fn points(&self, points: &[(f32, f32)]) -> Vec<Pos2> {
        points.iter().map(|&(x, y)| self.at(x, y)).collect()
    }

    fn line(&self, points: &[(f32, f32)]) {
        self.painter.add(PathShape::line(self.points(points), self.stroke));
    }

    fn closed(&self, points: &[(f32, f32)]) {
        self.painter.add(PathShape::closed_line(self.points(points), self.stroke));
    }

    fn circle(&self, x: f32, y: f32, radius: f32) {
        self.painter.circle_stroke(self.at(x, y), radius * self.size, self.stroke);
    }

    fn dot(&self, x: f32, y: f32, radius: f32) {
        self.painter.circle_filled(self.at(x, y), radius * self.size, self.stroke.color);
    }

    fn square(&self, half: f32, rounding: f32) {
        let side = 2.0 * half * self.size;
        self.painter.rect_stroke(
            Rect::from_center_size(self.center, vec2(side, side)),
            Rounding::same(rounding * self.size),
            self.stroke,
        );
    }
}

pub fn category(painter: &Painter, center: Pos2, size: f32, category: Category, color: Color32) {
    let pen = Pen::new(painter, center, size, color);
    match category {
        Category::UnsafeZone => {
            pen.circle(0.0, -0.12, 0.3);
            pen.line(&[(-0.27, 0.01), (0.0, 0.46), (0.27, 0.01)]);
            pen.dot(0.0, -0.12, 0.09);
        }
        Category::FacilityDamage => {
            pen.square(0.4, 0.12);
            pen.line(&[(0.06, -0.4), (-0.1, -0.12), (0.1, 0.08), (-0.04, 0.4)]);
        }
        Category::Emergency => {
            pen.circle(0.0, 0.0, 0.42);
            pen.line(&[(0.0, -0.2), (0.0, 0.2)]);
            pen.line(&[(-0.2, 0.0), (0.2, 0.0)]);
        }
        Category::Harassment => {
            pen.closed(&[(0.0, -0.46), (0.36, -0.32), (0.32, 0.1), (0.0, 0.46), (-0.32, 0.1), (-0.36, -0.32)]);
            pen.line(&[(0.0, -0.2), (0.0, 0.06)]);
            pen.dot(0.0, 0.19, 0.045);
        }
        Category::PoorLighting => {
            pen.circle(0.0, -0.1, 0.28);
            pen.line(&[(-0.12, 0.15), (-0.12, 0.27), (0.12, 0.27), (0.12, 0.15)]);
            pen.line(&[(-0.08, 0.39), (0.08, 0.39)]);
            pen.line(&[(-0.54, -0.1), (-0.44, -0.1)]);
            pen.line(&[(0.44, -0.1), (0.54, -0.1)]);
            pen.line(&[(0.0, -0.54), (0.0, -0.46)]);
        }
        Category::OtherRisk => warning(&pen),
    }
}

pub fn glyph(painter: &Painter, center: Pos2, size: f32, glyph: Glyph, color: Color32) {
    let pen = Pen::new(painter, center, size, color);
    match glyph {
        Glyph::Compose => {
            pen.square(0.38, 0.12);
            pen.line(&[(0.0, -0.17), (0.0, 0.17)]);
            pen.line(&[(-0.17, 0.0), (0.17, 0.0)]);
        }
        Glyph::List => {
            for y in [-0.24, 0.0, 0.24] {
                pen.dot(-0.32, y, 0.05);
                pen.line(&[(-0.14, y), (0.38, y)]);
            }
        }
        Glyph::Chart => {
            pen.line(&[(-0.4, 0.38), (0.4, 0.38)]);
            for (x, height) in [(-0.22, 0.32), (0.0, 0.62), (0.22, 0.46)] {
                pen.line(&[(x, 0.26), (x, 0.26 - height)]);
            }
        }
        Glyph::Alert => warning(&pen),
        Glyph::Check => {
            pen.circle(0.0, 0.0, 0.42);
            pen.line(&[(-0.18, 0.0), (-0.04, 0.14), (0.2, -0.12)]);
        }
        Glyph::Info => {
            pen.circle(0.0, 0.0, 0.42);
            pen.line(&[(0.0, -0.04), (0.0, 0.2)]);
            pen.dot(0.0, -0.18, 0.045);
        }
    }
}

fn warning(pen: &Pen<'_>) {
    pen.closed(&[(0.0, -0.42), (0.44, 0.36), (-0.44, 0.36)]);
    pen.line(&[(0.0, -0.1), (0.0, 0.12)]);
    pen.dot(0.0, 0.24, 0.045);
}

/// Logotipo: un faro visto desde arriba — anillos y un núcleo de luz.
pub fn logo(painter: &Painter, center: Pos2, size: f32, opacity: f32) {
    painter.add(radial(center, size * 1.3, |t| luminance(0.0, 0.22 * opacity * falloff(t), 1.0)));
    painter.circle_stroke(center, size * 0.5, Stroke::new(1.4, Color32::from_white_alpha(unit(0.55 * opacity))));
    painter.circle_stroke(center, size * 0.3, Stroke::new(1.2, Color32::from_white_alpha(unit(0.3 * opacity))));
    painter.circle_filled(center, size * 0.15, Color32::from_white_alpha(unit(0.95 * opacity)));
}
