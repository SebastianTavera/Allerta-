//! Notificaciones flotantes que entran deslizándose desde la derecha.

use std::time::Duration;

use eframe::egui::{pos2, vec2, Align2, Context, Id, LayerId, Order, Rect};

use super::fx::Fx;
use super::glass::{self, Glass};
use super::icons::{self, Glyph};
use super::motion::Motion;
use super::theme;
use super::widgets::{flag, text_line};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Success,
    Info,
    Warning,
}

impl Tone {
    fn glyph(self) -> Glyph {
        match self {
            Self::Success => Glyph::Check,
            Self::Info => Glyph::Info,
            Self::Warning => Glyph::Alert,
        }
    }
}

struct Toast {
    id: u64,
    tone: Tone,
    title: String,
    body: String,
    born: f64,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
    next_id: u64,
}

impl Toasts {
    const LIFETIME: f64 = 4.5;
    const MAX_VISIBLE: usize = 4;

    pub fn push(&mut self, tone: Tone, title: impl Into<String>, body: impl Into<String>, now: f64) {
        self.next_id += 1;
        self.items.push(Toast { id: self.next_id, tone, title: title.into(), body: body.into(), born: now });
        if self.items.len() > Self::MAX_VISIBLE {
            self.items.remove(0);
        }
    }

    pub fn show(&mut self, ctx: &Context, fx: &mut Fx, now: f64) {
        if self.items.is_empty() {
            return;
        }
        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("faro.toasts")));
        let screen = ctx.screen_rect();
        let width = 340.0_f32.min(screen.width() - 40.0);
        let height = 68.0;
        let mut bottom = screen.bottom() - 20.0;
        let mut finished = Vec::new();
        let mut next_deadline = f64::INFINITY;

        for toast in self.items.iter().rev() {
            let remaining = toast.born + Self::LIFETIME - now;
            let alive = remaining > 0.0;
            if alive {
                next_deadline = next_deadline.min(remaining);
            }
            let t = fx.springs.enter(Id::new(("toast", toast.id)), 0.0, flag(alive), Motion::Dramatic);
            if !alive && t < 0.02 {
                finished.push(toast.id);
                continue;
            }
            let visible = t.clamp(0.0, 1.0);
            let rect = Rect::from_min_size(
                pos2(screen.right() - 20.0 - width + (1.0 - t) * 70.0, bottom - height),
                vec2(width, height),
            );
            glass::paint(&painter, rect, &Glass::FLOATING, fx.light(), visible);

            let text = theme::TEXT_PRIMARY.gamma_multiply(visible);
            icons::glyph(&painter, rect.left_center() + vec2(30.0, 0.0), 20.0, toast.tone.glyph(), text);
            let text_left = rect.left() + 54.0;
            let text_width = rect.right() - 18.0 - text_left;
            text_line(&painter, pos2(text_left, rect.top() + 15.0), &toast.title, 14.0, text, text_width, Align2::LEFT_TOP);
            text_line(
                &painter,
                pos2(text_left, rect.bottom() - 15.0),
                &toast.body,
                12.5,
                theme::TEXT_SECONDARY.gamma_multiply(visible),
                text_width,
                Align2::LEFT_BOTTOM,
            );
            bottom -= (height + 10.0) * visible;
        }

        self.items.retain(|toast| !finished.contains(&toast.id));
        if next_deadline.is_finite() {
            ctx.request_repaint_after(Duration::from_secs_f64(next_deadline.max(0.0) + 0.02));
        }
    }
}
