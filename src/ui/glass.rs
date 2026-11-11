//! Superficies de "vidrio líquido".
//!
//! Cada superficie se teselea como una malla en abanico centrada en la luz:
//! así el brillo especular se calcula por vértice, queda recortado con
//! precisión a la forma redondeada y cuesta apenas unos cientos de vértices.
//! El borde es una tira con bisel: más brillante arriba (luz cenital) y donde
//! la luz del puntero lo roza.

use std::f32::consts::TAU;

use eframe::egui::epaint::{Mesh, Shadow};
use eframe::egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Rounding, Shape};

use super::light::LightSource;
use super::theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glass {
    pub radius: f32,
    /// Opacidad del velo blanco del cristal.
    pub tint: f32,
    /// Intensidad del borde.
    pub edge: f32,
    /// Cuánto refleja la luz del puntero.
    pub specular: f32,
    pub shadow: f32,
    /// Oscurecimiento detrás del vidrio (para superficies flotantes).
    pub backdrop: f32,
    /// Radio de influencia de la luz, en puntos.
    pub reach: f32,
}

impl Glass {
    pub const PANEL: Self = Self {
        radius: theme::RADIUS_PANEL,
        tint: 0.045,
        edge: 0.17,
        specular: 1.0,
        shadow: 0.28,
        backdrop: 0.0,
        reach: 280.0,
    };
    pub const CARD: Self = Self {
        radius: theme::RADIUS_CARD,
        tint: 0.038,
        edge: 0.13,
        specular: 0.85,
        shadow: 0.18,
        backdrop: 0.0,
        reach: 220.0,
    };
    pub const CONTROL: Self = Self {
        radius: theme::RADIUS_CONTROL,
        tint: 0.035,
        edge: 0.12,
        specular: 0.7,
        shadow: 0.0,
        backdrop: 0.0,
        reach: 150.0,
    };
    pub const PILL: Self = Self {
        radius: theme::RADIUS_CONTROL,
        tint: 0.10,
        edge: 0.34,
        specular: 1.0,
        shadow: 0.16,
        backdrop: 0.0,
        reach: 150.0,
    };
    pub const SOLID: Self = Self {
        radius: 22.0,
        tint: 0.86,
        edge: 0.5,
        specular: 0.5,
        shadow: 0.22,
        backdrop: 0.0,
        reach: 120.0,
    };
    pub const FLOATING: Self = Self {
        radius: theme::RADIUS_CARD,
        tint: 0.07,
        edge: 0.22,
        specular: 0.9,
        shadow: 0.35,
        backdrop: 0.72,
        reach: 200.0,
    };

    pub fn radius(self, radius: f32) -> Self {
        Self { radius, ..self }
    }

    pub fn tint(self, tint: f32) -> Self {
        Self { tint, ..self }
    }

    pub fn edge(self, edge: f32) -> Self {
        Self { edge, ..self }
    }

    pub fn shadow(self, shadow: f32) -> Self {
        Self { shadow, ..self }
    }
}

pub fn paint(painter: &Painter, rect: Rect, glass: &Glass, light: LightSource, opacity: f32) {
    painter.add(shape(rect, glass, light, opacity));
}

pub fn shape(rect: Rect, glass: &Glass, light: LightSource, opacity: f32) -> Shape {
    let opacity = opacity.clamp(0.0, 1.0);
    if opacity < 0.004 || !rect.is_positive() {
        return Shape::Noop;
    }
    let radius = glass.radius.min(rect.width() * 0.5).min(rect.height() * 0.5).max(0.0);
    let outline = rounded_outline(rect, radius);

    let mut shapes = Vec::with_capacity(4);
    if glass.shadow > 0.0 {
        let shadow = Shadow {
            offset: vec2(0.0, 12.0),
            blur: 34.0,
            spread: 0.0,
            color: Color32::from_black_alpha(unit(glass.shadow * opacity)),
        };
        shapes.push(Shape::from(shadow.as_shape(rect, Rounding::same(radius))));
    }
    if glass.backdrop > 0.0 {
        shapes.push(Shape::rect_filled(
            rect,
            Rounding::same(radius),
            theme::BACKGROUND.gamma_multiply(glass.backdrop * opacity),
        ));
    }
    shapes.push(Shape::mesh(body(rect, &outline, radius, glass, light, opacity)));
    shapes.push(Shape::mesh(rim(rect, &outline, glass, light, opacity)));
    Shape::Vec(shapes)
}

/// Fondo: negro profundo con orbes grises difusos que se desplazan con
/// paralaje según la luz. Dan al vidrio algo que "refractar".
pub fn background(painter: &Painter, screen: Rect, light: LightSource) {
    painter.rect_filled(screen, 0.0, theme::BACKGROUND);
    let size = screen.size();
    let span = size.x.max(size.y);
    let drift = (light.pos - screen.center()) * light.intensity;

    // (x, y, radio, intensidad, profundidad de paralaje)
    const ORBS: [(f32, f32, f32, f32, f32); 4] = [
        (0.16, 0.12, 0.52, 0.13, -0.030),
        (0.86, 0.26, 0.40, 0.09, -0.055),
        (0.58, 1.05, 0.62, 0.11, -0.020),
        (0.40, 0.55, 0.22, 0.05, -0.085),
    ];
    for (x, y, radius, strength, depth) in ORBS {
        let center = screen.min + vec2(x * size.x, y * size.y) + drift * depth;
        painter.add(radial(center, radius * span, |t| luminance(strength * falloff(t), 0.0, 1.0)));
    }
    // La luz del puntero ilumina levemente el "suelo".
    painter.add(radial(light.pos, 520.0, |t| {
        luminance(0.0, 0.05 * light.intensity * falloff(t), 1.0)
    }));
}

// ───────────────────────────── Primitivas ─────────────────────────────

const RAYS: [f32; 9] = [0.0, 0.07, 0.16, 0.27, 0.4, 0.54, 0.69, 0.84, 1.0];

fn body(rect: Rect, outline: &[Pos2], radius: f32, glass: &Glass, light: LightSource, opacity: f32) -> Mesh {
    // El centro del abanico debe quedar dentro de la forma convexa.
    let core = rect.shrink(radius);
    let center = pos2(
        light.pos.x.max(core.left()).min(core.right()),
        light.pos.y.max(core.top()).min(core.bottom()),
    );

    let rings = RAYS.len() as u32;
    let mut mesh = Mesh::default();
    for &edge in outline {
        for &t in &RAYS {
            let p = center + (edge - center) * t;
            mesh.colored_vertex(p, body_color(p, rect, glass, light, opacity));
        }
    }
    let n = outline.len() as u32;
    for i in 0..n {
        let j = (i + 1) % n;
        for k in 0..rings - 1 {
            let a = i * rings + k;
            let b = j * rings + k;
            mesh.add_triangle(a, b, a + 1);
            mesh.add_triangle(b, b + 1, a + 1);
        }
    }
    mesh
}

fn body_color(p: Pos2, rect: Rect, glass: &Glass, light: LightSource, opacity: f32) -> Color32 {
    let depth = ((p.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
    let sheen = 0.045 * (1.0 - depth).powi(3);
    let d2 = p.distance_sq(light.pos);
    let broad = gaussian(d2, glass.reach);
    let sharp = gaussian(d2, glass.reach * 0.32);
    let spec = glass.specular * light.intensity;
    luminance(
        glass.tint + sheen + spec * (0.035 * broad + 0.05 * sharp),
        spec * (0.10 * broad + 0.10 * sharp),
        opacity,
    )
}

fn rim(rect: Rect, outline: &[Pos2], glass: &Glass, light: LightSource, opacity: f32) -> Mesh {
    // Desplazamientos respecto al contorno (positivo = hacia afuera):
    // difuminado exterior, línea, bisel interior y desvanecimiento.
    const OFFSETS: [f32; 5] = [1.5, 0.5, -0.5, -2.5, -6.0];
    let n = outline.len();
    let mut mesh = Mesh::default();

    for i in 0..n {
        let p = outline[i];
        let tangent = (outline[(i + 1) % n] - outline[(i + n - 1) % n]).normalized();
        let normal = vec2(tangent.y, -tangent.x);
        let depth = ((p.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
        let glint = glass.specular * light.intensity * gaussian(p.distance_sq(light.pos), glass.reach * 0.7);
        let line = glass.edge * (0.4 + 0.6 * (1.0 - depth)) + glint * 0.8;
        let bevel = glass.edge * 0.18 * (1.0 - depth).powi(2) + glint * 0.18;

        let line_color = luminance(line * 0.55, line * 0.45, opacity);
        let colors = [
            Color32::TRANSPARENT,
            line_color,
            line_color,
            luminance(bevel * 0.4, bevel * 0.6, opacity),
            Color32::TRANSPARENT,
        ];
        for (offset, color) in OFFSETS.iter().zip(colors) {
            mesh.colored_vertex(p + normal * *offset, color);
        }
    }

    let columns = OFFSETS.len() as u32;
    let n = n as u32;
    for i in 0..n {
        let j = (i + 1) % n;
        for k in 0..columns - 1 {
            let a = i * columns + k;
            let b = j * columns + k;
            mesh.add_triangle(a, b, a + 1);
            mesh.add_triangle(b, b + 1, a + 1);
        }
    }
    mesh
}

/// Contorno de un rectángulo redondeado en sentido horario.
fn rounded_outline(rect: Rect, radius: f32) -> Vec<Pos2> {
    let segments = ((radius / 2.5).ceil() as usize).clamp(2, 14);
    let corners = [
        (pos2(rect.right() - radius, rect.top() + radius), -90.0_f32),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.0),
        (pos2(rect.left() + radius, rect.bottom() - radius), 90.0),
        (pos2(rect.left() + radius, rect.top() + radius), 180.0),
    ];
    let mut points = Vec::with_capacity(4 * (segments + 1));
    for (center, start) in corners {
        for s in 0..=segments {
            let angle = (start + 90.0 * s as f32 / segments as f32).to_radians();
            points.push(center + vec2(angle.cos(), angle.sin()) * radius);
        }
    }
    points
}

/// Gradiente radial genérico; `color` recibe la distancia normalizada 0‥1.
pub fn radial(center: Pos2, radius: f32, color: impl Fn(f32) -> Color32) -> Shape {
    const RINGS: [f32; 8] = [0.0, 0.12, 0.25, 0.4, 0.55, 0.7, 0.85, 1.0];
    const SEGMENTS: u32 = 40;

    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color(0.0));
    for &t in &RINGS[1..] {
        let ring_color = color(t);
        for s in 0..SEGMENTS {
            let angle = s as f32 / SEGMENTS as f32 * TAU;
            mesh.colored_vertex(center + vec2(angle.cos(), angle.sin()) * radius * t, ring_color);
        }
    }
    for s in 0..SEGMENTS {
        mesh.add_triangle(0, 1 + s, 1 + (s + 1) % SEGMENTS);
    }
    for ring in 0..(RINGS.len() as u32 - 2) {
        let base = 1 + ring * SEGMENTS;
        let next = base + SEGMENTS;
        for s in 0..SEGMENTS {
            let s1 = (s + 1) % SEGMENTS;
            mesh.add_triangle(base + s, base + s1, next + s);
            mesh.add_triangle(base + s1, next + s1, next + s);
        }
    }
    Shape::mesh(mesh)
}

/// Color premultiplicado blanco: `cover` se mezcla normalmente y `glow` se
/// suma (luz aditiva), lo que produce reflejos verosímiles.
pub fn luminance(cover: f32, glow: f32, opacity: f32) -> Color32 {
    let alpha = (cover * opacity).clamp(0.0, 1.0);
    let value = ((cover + glow) * opacity).clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(unit(value), unit(value), unit(value), unit(alpha))
}

/// Caída suave de 1 (centro) a 0 (borde).
pub fn falloff(t: f32) -> f32 {
    (1.0 - t * t).max(0.0).powi(3)
}

pub fn unit(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn gaussian(distance_sq: f32, reach: f32) -> f32 {
    (-distance_sq / (reach * reach)).exp()
}
