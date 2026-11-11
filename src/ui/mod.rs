//! Capa de presentación: interfaz de "vidrio líquido" monocromática.
//!
//! El rendimiento se cuida de forma explícita: la ventana solo se redibuja
//! cuando hay entrada del usuario o una animación en curso; en reposo el
//! consumo de CPU es prácticamente cero.

mod app;
mod fx;
mod glass;
mod icons;
mod light;
mod motion;
mod pages;
mod theme;
mod toast;
mod widgets;

pub use app::FaroApp;

use eframe::egui;

pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Faro · Seguridad en el campus")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([640.0, 520.0]),
        renderer: eframe::Renderer::Glow,
        vsync: true,
        multisampling: 0,
        ..Default::default()
    }
}
