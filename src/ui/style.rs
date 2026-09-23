/* ui/style.rs
 *
 * Colores y estilos reutilizables para tarjetas, chips y mensajes
 * de estado. Todo lo visual "de bajo nivel" vive aquí para que
 * app.rs y components.rs no se llenen de structs de Border/Shadow.
 */

use iced::widget::container;
use iced::{Background, Border, Color, Shadow, Theme};

pub fn muted_color() -> Color {
    Color::from_rgb(0.6, 0.6, 0.63)
}

pub fn chip_blue() -> Color {
    Color::from_rgb(0.24, 0.4, 0.75)
}

pub fn chip_purple() -> Color {
    Color::from_rgb(0.45, 0.32, 0.65)
}

pub fn chip_gray() -> Color {
    Color::from_rgb(0.35, 0.35, 0.38)
}

pub fn chip_green() -> Color {
    Color::from_rgb(0.25, 0.55, 0.35)
}

pub fn chip_red() -> Color {
    Color::from_rgb(0.6, 0.28, 0.28)
}

#[derive(Debug, Clone)]
pub enum StatusKind {
    Info,
    Ok,
    Err,
}

pub fn status_color(kind: &StatusKind) -> Color {
    match kind {
        StatusKind::Info => Color::from_rgb(0.75, 0.75, 0.8),
        StatusKind::Ok => Color::from_rgb(0.55, 0.85, 0.6),
        StatusKind::Err => Color::from_rgb(0.95, 0.5, 0.5),
    }
}

pub fn status_style(_theme: &Theme, kind: &StatusKind) -> container::Style {
    let bg = match kind {
        StatusKind::Info => Color::from_rgba(0.5, 0.5, 0.55, 0.12),
        StatusKind::Ok => Color::from_rgba(0.3, 0.7, 0.35, 0.12),
        StatusKind::Err => Color::from_rgba(0.8, 0.3, 0.3, 0.12),
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.13, 0.13, 0.15))),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color::from_rgb(0.22, 0.22, 0.25),
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

/// Tarjeta chica anidada dentro de otra (p. ej. cada app listada
/// dentro de la tarjeta de su prefijo). Un poco más clara que la
/// tarjeta contenedora para diferenciarlas.
pub fn subcard_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.17, 0.17, 0.19))),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: Color::from_rgb(0.25, 0.25, 0.28),
        },
        ..Default::default()
    }
}

pub fn chip_style(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Paleta fija para las miniaturas de la biblioteca. Cada app tiene
/// siempre el mismo color, elegido a partir de su nombre, para que
/// sea fácil reconocerlas de un vistazo sin necesitar el ícono real
/// del .exe (extraerlo de un PE de Windows es un desarrollo aparte).
fn thumbnail_palette() -> [Color; 7] {
    [
        Color::from_rgb(0.24, 0.4, 0.75),
        Color::from_rgb(0.45, 0.32, 0.65),
        Color::from_rgb(0.25, 0.55, 0.35),
        Color::from_rgb(0.75, 0.45, 0.2),
        Color::from_rgb(0.6, 0.28, 0.28),
        Color::from_rgb(0.2, 0.55, 0.6),
        Color::from_rgb(0.55, 0.5, 0.2),
    ]
}

pub fn thumbnail_color(seed: &str) -> Color {
    let hash: u32 = seed.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let palette = thumbnail_palette();
    palette[(hash as usize) % palette.len()]
}
