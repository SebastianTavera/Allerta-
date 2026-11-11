//! Contexto de efectos compartido por todos los widgets de un cuadro:
//! resortes de animación y la luz del puntero.

use eframe::egui::{Context, CursorIcon};

use super::light::{Light, LightSource};
use super::motion::SpringBank;

#[derive(Default)]
pub struct Fx {
    pub springs: SpringBank,
    light: Light,
}

impl Fx {
    pub fn begin_frame(&mut self, ctx: &Context) {
        let dt = ctx.input(|i| i.stable_dt).clamp(1.0 / 240.0, 1.0 / 20.0);
        self.springs.begin_frame(dt);
        self.light.update(ctx, dt);
    }

    pub fn light(&self) -> LightSource {
        self.light.source()
    }

    /// Sustituye el cursor del sistema por la luz y pide otro cuadro solo
    /// si algo sigue animándose.
    pub fn end_frame(&mut self, ctx: &Context) {
        let icon = ctx.output(|o| o.cursor_icon);
        if matches!(icon, CursorIcon::Default | CursorIcon::PointingHand) {
            ctx.set_cursor_icon(CursorIcon::None);
        }
        self.light.set_hover(icon == CursorIcon::PointingHand);
        self.light.paint(ctx);

        if self.springs.is_active() || self.light.is_moving() {
            ctx.request_repaint();
        }
    }
}
