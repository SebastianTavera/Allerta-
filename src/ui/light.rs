//! El puntero es una fuente de luz: un núcleo brillante con un halo que lo
//! sigue con inercia. Su posición alimenta los reflejos de todo el vidrio.

use eframe::egui::{pos2, Color32, Context, Id, LayerId, Order, Pos2, Stroke};

use super::glass::{falloff, luminance, radial, unit};
use super::motion::{Motion, Spring, PIXEL_PRECISION, UNIT_PRECISION};

/// Lo que el vidrio necesita saber de la luz para reflejarla.
#[derive(Clone, Copy, Debug)]
pub struct LightSource {
    pub pos: Pos2,
    /// 0 cuando el puntero está fuera de la ventana, 1 cuando está presente.
    pub intensity: f32,
}

pub struct Light {
    pointer: Pos2,
    halo_x: Spring,
    halo_y: Spring,
    presence: Spring,
    hover: Spring,
    pulse: Spring,
    seen: bool,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            pointer: pos2(-1000.0, -1000.0),
            halo_x: Motion::Fluid.spring(-1000.0, PIXEL_PRECISION),
            halo_y: Motion::Fluid.spring(-1000.0, PIXEL_PRECISION),
            presence: Motion::Smooth.spring(0.0, UNIT_PRECISION),
            hover: Motion::Dramatic.spring(0.0, UNIT_PRECISION),
            pulse: Motion::Smooth.spring(0.0, UNIT_PRECISION),
            seen: false,
        }
    }
}

impl Light {
    pub fn update(&mut self, ctx: &Context, dt: f32) {
        let (hover, pressed) = ctx.input(|i| (i.pointer.hover_pos(), i.pointer.any_pressed()));
        match hover {
            Some(pos) => {
                if !self.seen {
                    self.halo_x.snap(pos.x);
                    self.halo_y.snap(pos.y);
                    self.seen = true;
                }
                self.pointer = pos;
                self.halo_x.target = pos.x;
                self.halo_y.target = pos.y;
                self.presence.target = 1.0;
            }
            None => self.presence.target = 0.0,
        }
        if pressed {
            self.pulse.kick(1.0, 0.0);
        }
        for spring in self.springs_mut() {
            spring.step(dt);
        }
    }

    /// Indica si el puntero está sobre algo interactivo (la luz se expande).
    pub fn set_hover(&mut self, interactive: bool) {
        self.hover.target = if interactive { 1.0 } else { 0.0 };
    }

    pub fn is_moving(&self) -> bool {
        [&self.halo_x, &self.halo_y, &self.presence, &self.hover, &self.pulse]
            .iter()
            .any(|spring| !spring.is_resting())
    }

    pub fn source(&self) -> LightSource {
        LightSource {
            pos: pos2(self.halo_x.value, self.halo_y.value),
            intensity: self.presence.value.clamp(0.0, 1.0),
        }
    }

    pub fn paint(&self, ctx: &Context) {
        let presence = self.presence.value.clamp(0.0, 1.0);
        if presence < 0.01 {
            return;
        }
        let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("faro.light")));
        let hover = self.hover.value.max(0.0);
        let halo = pos2(self.halo_x.value, self.halo_y.value);

        // Resplandor amplio que llega con retraso: sensación de luz física.
        painter.add(radial(halo, 110.0 + 40.0 * hover, |t| {
            luminance(0.0, 0.06 * presence * falloff(t), 1.0)
        }));
        // Bloom alrededor del núcleo.
        painter.add(radial(self.pointer, 20.0 + 12.0 * hover, |t| {
            let f = falloff(t);
            luminance(0.08 * presence * f, 0.5 * presence * f, 1.0)
        }));
        painter.circle_filled(
            self.pointer,
            2.4 + 1.8 * hover,
            Color32::from_white_alpha(unit(0.96 * presence)),
        );

        // Onda expansiva al hacer clic.
        let pulse = self.pulse.value.clamp(0.0, 1.0);
        if pulse > 0.01 {
            painter.circle_stroke(
                self.pointer,
                5.0 + (1.0 - pulse) * 28.0,
                Stroke::new(1.2_f32, Color32::from_white_alpha(unit(0.5 * pulse * presence))),
            );
        }
    }

    fn springs_mut(&mut self) -> [&mut Spring; 5] {
        [&mut self.halo_x, &mut self.halo_y, &mut self.presence, &mut self.hover, &mut self.pulse]
    }
}
