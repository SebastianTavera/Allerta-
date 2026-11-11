//! Animación física con resortes amortiguados.
//!
//! A diferencia de las curvas de tiempo fijo, un resorte conserva su
//! velocidad cuando cambia el destino a mitad de camino: los movimientos se
//! sienten continuos, con inercia y un ligero rebote — "casi reales".

use std::collections::HashMap;

use eframe::egui::Id;

pub const UNIT_PRECISION: f32 = 0.0015;
pub const PIXEL_PRECISION: f32 = 0.08;

/// Carácter del movimiento.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// Lento, con inercia y un rebote visible. Transiciones principales.
    Dramatic,
    /// Suave y sin rebote apreciable. Estados de hover y foco.
    Smooth,
    /// Rápido y firme. Respuesta a pulsaciones.
    Snappy,
    /// Seguimiento ágil, para la luz del puntero.
    Fluid,
}

impl Motion {
    /// (rigidez, razón de amortiguamiento)
    fn parameters(self) -> (f32, f32) {
        match self {
            Self::Dramatic => (68.0, 0.62),
            Self::Smooth => (130.0, 0.92),
            Self::Snappy => (380.0, 0.86),
            Self::Fluid => (260.0, 0.84),
        }
    }

    pub fn spring(self, value: f32, precision: f32) -> Spring {
        let (stiffness, damping_ratio) = self.parameters();
        Spring {
            value,
            velocity: 0.0,
            target: value,
            stiffness,
            damping: 2.0 * damping_ratio * stiffness.sqrt(),
            precision,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
    stiffness: f32,
    damping: f32,
    precision: f32,
}

impl Spring {
    const MAX_STEP: f32 = 1.0 / 240.0;

    /// Integra el resorte (Euler semi-implícito con sub-pasos estables).
    pub fn step(&mut self, dt: f32) {
        if self.is_resting() {
            self.value = self.target;
            self.velocity = 0.0;
            return;
        }
        let mut remaining = dt.clamp(0.0, 0.1);
        while remaining > 0.0 {
            let h = remaining.min(Self::MAX_STEP);
            let acceleration =
                -self.stiffness * (self.value - self.target) - self.damping * self.velocity;
            self.velocity += acceleration * h;
            self.value += self.velocity * h;
            remaining -= h;
        }
    }

    pub fn is_resting(&self) -> bool {
        (self.value - self.target).abs() <= self.precision
            && self.velocity.abs() <= self.precision * 8.0
    }

    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    /// Reinicia el resorte en `from` en camino hacia `to`.
    pub fn kick(&mut self, from: f32, to: f32) {
        self.value = from;
        self.target = to;
        self.velocity = 0.0;
    }
}

struct Entry {
    spring: Spring,
    stepped_frame: u64,
}

/// Almacén de resortes indexados por `Id`, avanzados una vez por cuadro.
/// Informa si alguno sigue en movimiento para pedir el siguiente cuadro.
#[derive(Default)]
pub struct SpringBank {
    entries: HashMap<Id, Entry>,
    frame: u64,
    dt: f32,
    active: bool,
}

impl SpringBank {
    const GC_INTERVAL: u64 = 900;

    pub fn begin_frame(&mut self, dt: f32) {
        self.frame += 1;
        self.dt = dt;
        self.active = false;
        if self.frame % Self::GC_INTERVAL == 0 {
            let frame = self.frame;
            self.entries.retain(|_, entry| frame - entry.stepped_frame < Self::GC_INTERVAL);
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Valor normalizado que nace en reposo sobre `target`.
    pub fn value(&mut self, id: Id, target: f32, motion: Motion) -> f32 {
        self.drive(id, target, target, motion, UNIT_PRECISION).value
    }

    /// Valor normalizado que nace en `initial` y viaja hacia `target`.
    pub fn enter(&mut self, id: Id, initial: f32, target: f32, motion: Motion) -> f32 {
        self.drive(id, initial, target, motion, UNIT_PRECISION).value
    }

    /// Resorte en píxeles (posiciones, tamaños). Devuelve valor y velocidad.
    pub fn pixels(&mut self, id: Id, target: f32, motion: Motion) -> Spring {
        self.drive(id, target, target, motion, PIXEL_PRECISION)
    }

    /// Contador que sube desde cero con rebote.
    pub fn count(&mut self, id: Id, target: f32) -> f32 {
        self.drive(id, 0.0, target, Motion::Dramatic, PIXEL_PRECISION).value
    }

    pub fn forget(&mut self, id: Id) {
        self.entries.remove(&id);
    }

    fn drive(&mut self, id: Id, initial: f32, target: f32, motion: Motion, precision: f32) -> Spring {
        let frame = self.frame;
        let entry = self
            .entries
            .entry(id)
            .or_insert_with(|| Entry { spring: motion.spring(initial, precision), stepped_frame: 0 });
        entry.spring.target = target;
        if entry.stepped_frame != frame {
            entry.spring.step(self.dt);
            entry.stepped_frame = frame;
        }
        if !entry.spring.is_resting() {
            self.active = true;
        }
        entry.spring
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_settles_on_target() {
        let mut spring = Motion::Dramatic.spring(0.0, UNIT_PRECISION);
        spring.target = 1.0;
        let mut overshoot = false;
        for _ in 0..600 {
            spring.step(1.0 / 60.0);
            overshoot |= spring.value > 1.0;
        }
        assert!(overshoot, "el movimiento dramático debe rebotar levemente");
        assert!(spring.is_resting());
        assert_eq!(spring.value, 1.0);
    }

    #[test]
    fn bank_reports_activity_until_rest() {
        let mut bank = SpringBank::default();
        let id = Id::new("test");
        bank.begin_frame(1.0 / 60.0);
        bank.enter(id, 0.0, 1.0, Motion::Smooth);
        assert!(bank.is_active());
        for _ in 0..600 {
            bank.begin_frame(1.0 / 60.0);
            bank.enter(id, 0.0, 1.0, Motion::Smooth);
        }
        assert!(!bank.is_active());
    }
}
