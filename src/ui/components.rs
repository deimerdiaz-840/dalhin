/* ui/components.rs
 *
 * Piezas visuales reutilizables sin estado propio: reciben datos
 * "de solo lectura" (&Prefix, &SystemReport...) y devuelven un
 * Element. Toda la lógica de qué hacer al hacer clic vive en
 * app.rs, vía los Message que estos widgets emiten.
 */

use crate::core::app::Message;
use crate::i18n::{self, Lang};
use crate::core::prefix::{DetectedPrefix, Prefix};
use crate::core::system::SystemReport;
use crate::ui::style::{self, StatusKind};
use iced::widget::image::Handle as ImageHandle;
use iced::widget::{button, column, container, image, row, text};
use iced::{Alignment, Color, Element, Length};
use std::collections::HashMap;

pub fn chip(label: &str, color: Color) -> Element<'_, Message> {
    container(text(label.to_string()).size(11).color(Color::WHITE))
        .padding([2, 8])
        .style(style::chip_style(color))
        .into()
}

pub fn status_banner(status: &Option<(StatusKind, String)>) -> Element<'_, Message> {
    match status {
        Some((kind, msg)) => {
            let color = style::status_color(kind);
            container(text(msg.clone()).size(14).color(color))
                .padding([8, 12])
                .style(move |t| style::status_style(t, kind))
                .into()
        }
        None => column![].into(),
    }
}

pub fn prefix_row<'a>(
    p: &'a Prefix,
    awaiting_scan: bool,
    icon_cache: &HashMap<String, Option<ImageHandle>>,
    lang: Lang,
) -> Element<'a, Message> {
    let t = i18n::tr(lang);
    let chips = row![chip(&p.arch, style::chip_blue()), chip(&p.category, style::chip_purple())]
        .spacing(6);
    let chips = if p.external {
        chips.push(chip(t.external, style::chip_gray()))
    } else {
        chips
    };

    let file_label = |exe: &str| {
        std::path::Path::new(exe)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| exe.to_string())
    };

    let mut subtitle_lines = column![].spacing(2);
    if let Some(exe) = &p.setup_exe {
        subtitle_lines = subtitle_lines.push(
            text(i18n::msg_installer_label(lang, &file_label(exe)))
                .size(11)
                .color(style::muted_color()),
        );
    }

    // Botón para agregar/cambiar el instalador: siempre disponible,
    // solo guarda cuál es — no lo corre. Correrlo es una acción
    // aparte (el botón "Instalar" de abajo).
    let setup_button = button(
        text(if p.setup_exe.is_some() { t.change_installer } else { t.add_installer }).size(12),
    )
    .on_press(Message::ChangeSetupPressed(p.name.clone()))
    .padding([4, 10])
    .style(button::text);

    // "Instalar" solo se puede tocar cuando ya hay un instalador
    // agregado — si no, el botón queda deshabilitado (sin
    // on_press) para que no se confunda con "elegir instalador".
    let mut install_button = button(text(t.install).size(13)).padding([6, 12]).style(button::secondary);
    if p.setup_exe.is_some() {
        install_button = install_button.on_press(Message::InstallPressed(p.name.clone()));
    }

    // Fila 1: acciones del prefijo en sí. No dependen de cuántas
    // apps tenga adentro.
    let prefix_actions = row![
        setup_button,
        install_button,
        button(text(t.winecfg).size(12))
            .on_press(Message::WinecfgPressed(p.name.clone()))
            .padding([4, 10])
            .style(button::text),
        button(text(t.delete).size(12))
            .on_press(Message::DeletePressed(p.name.clone()))
            .padding([4, 10])
            .style(button::danger),
    ]
    .spacing(8);

    // Lista de apps de este prefijo, cada una con su propio botón
    // "Abrir" al lado — así, si hay varias, se abre primero una y
    // después la otra sin tener que "cambiar" cuál es la principal.
    // Incluye también `launcher_exe` si por alguna razón no está en
    // la biblioteca (prefijos de versiones anteriores de Dalhin).
    let mut apps: Vec<(String, String)> =
        p.library.iter().map(|a| (a.name.clone(), a.exe_path.clone())).collect();
    if let Some(exe) = &p.launcher_exe {
        if !apps.iter().any(|(_, path)| path == exe) {
            apps.push((file_label(exe), exe.clone()));
        }
    }

    let mut app_list = column![].spacing(6);
    if apps.is_empty() {
        let hint = if awaiting_scan { t.installing_hint } else { t.no_apps_yet };
        app_list = app_list.push(text(hint).size(11).color(style::muted_color()));
    } else {
        for (name, exe_path) in &apps {
            let icon = icon_cache.get(exe_path).cloned().flatten();
            let row_item = row![
                app_thumbnail(name, icon),
                text(name.clone()).size(13),
                iced::widget::horizontal_space(),
                button(text(t.open).size(12))
                    .on_press(Message::LaunchAppPressed(p.name.clone(), exe_path.clone()))
                    .padding([4, 10])
                    .style(button::success),
                button(text(t.remove).size(11))
                    .on_press(Message::RemoveAppPressed(p.name.clone(), exe_path.clone()))
                    .padding([4, 10])
                    .style(button::text),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            app_list = app_list.push(
                container(row_item)
                    .padding([6, 10])
                    .width(Length::Fill)
                    .style(style::subcard_style),
            );
        }
    }

    // Fila 2: conseguir más apps para este prefijo. "Buscar
    // aplicaciones instaladas" escanea Escritorio y Menú Inicio (como
    // hace Bottles) y las agrega solas a la lista de arriba.
    let scan_label = if awaiting_scan { t.scan_done_button } else { t.scan_installed_apps };
    let discover_actions = row![
        button(text(scan_label).size(12))
            .on_press(Message::ScanProgramsPressed(p.name.clone()))
            .padding([4, 10])
            .style(if awaiting_scan { button::success } else { button::text }),
        button(text(t.add_to_library).size(12))
            .on_press(Message::AddAppPressed(p.name.clone()))
            .padding([4, 10])
            .style(button::text),
    ]
    .spacing(8);

    container(
        column![
            row![text(&p.name).size(15), chips].spacing(10).align_y(Alignment::Center),
            subtitle_lines,
            prefix_actions.align_y(Alignment::Center),
            app_list,
            discover_actions.align_y(Alignment::Center),
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Length::Fill)
    .style(style::card_style)
    .into()
}


/// Miniatura de una app de la biblioteca: si ya se extrajo el ícono
/// real del .exe (ver `icon.rs`), lo muestra; si no (todavía no se
/// terminó de cargar, o el .exe no tiene ícono embebido), cae en un
/// avatar de color fijo por app con la inicial, para poder
/// reconocerlas de un vistazo igual.
pub fn app_thumbnail(name: &str, icon: Option<ImageHandle>) -> Element<'static, Message> {
    if let Some(handle) = icon {
        return container(image(handle).width(Length::Fixed(32.0)).height(Length::Fixed(32.0)))
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .center_x(Length::Fixed(36.0))
            .center_y(Length::Fixed(36.0))
            .into();
    }
    let letter = name.chars().next().unwrap_or('?').to_uppercase().to_string();
    let color = style::thumbnail_color(name);
    container(text(letter).size(16).color(Color::WHITE))
        .width(Length::Fixed(36.0))
        .height(Length::Fixed(36.0))
        .center_x(Length::Fixed(36.0))
        .center_y(Length::Fixed(36.0))
        .style(style::chip_style(color))
        .into()
}

/// Vista de la pestaña "Biblioteca": junta las apps agregadas en
/// todos los prefijos (no solo uno) para tenerlas todas a mano en
/// un solo lugar, con miniatura, nombre, a qué prefijo pertenecen y
/// su propio botón de Abrir. Ocupa todo el ancho disponible, ya que
/// ahora es una pestaña propia y no un panel lateral.
pub fn library_view<'a>(
    prefixes: &[Prefix],
    icon_cache: &HashMap<String, Option<ImageHandle>>,
    lang: Lang,
) -> Element<'a, Message> {
    let t = i18n::tr(lang);
    let mut entries: Vec<(&Prefix, &crate::core::prefix::LibraryApp)> = Vec::new();
    for p in prefixes {
        for app in &p.library {
            entries.push((p, app));
        }
    }

    let body: Element<'_, Message> = if entries.is_empty() {
        container(
            text(t.library_empty)
                .size(13)
                .color(style::muted_color()),
        )
        .padding(24)
        .width(Length::Fill)
        .style(style::card_style)
        .into()
    } else {
        entries
            .into_iter()
            .fold(column![].spacing(8), |col, (p, app)| {
                let icon = icon_cache.get(&app.exe_path).cloned().flatten();
                let row_item = row![
                    app_thumbnail(&app.name, icon),
                    column![
                        text(app.name.clone()).size(14),
                        text(p.name.clone()).size(11).color(style::muted_color()),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(text(t.open).size(13))
                        .on_press(Message::LaunchAppPressed(p.name.clone(), app.exe_path.clone()))
                        .padding([6, 12])
                        .style(button::success),
                    button(text(t.remove).size(12))
                        .on_press(Message::RemoveAppPressed(p.name.clone(), app.exe_path.clone()))
                        .padding([6, 12])
                        .style(button::text),
                ]
                .spacing(10)
                .align_y(Alignment::Center);
                col.push(
                    container(row_item)
                        .padding(10)
                        .width(Length::Fill)
                        .style(style::card_style),
                )
            })
            .into()
    };

    let count = prefixes.iter().map(|p| p.library.len()).sum::<usize>();
    column![text(i18n::msg_library_count(lang, count)).size(16), body]
        .spacing(12)
        .into()
}

pub fn detected_row(d: &DetectedPrefix, lang: Lang) -> Element<'_, Message> {
    let t = i18n::tr(lang);
    row![
        column![
            text(&d.name).size(14),
            text(&d.path).size(11).color(style::muted_color()),
        ]
        .spacing(2),
        iced::widget::horizontal_space(),
        chip(&d.arch, style::chip_blue()),
        button(text(t.add).size(13))
            .on_press(Message::AddDetectedPressed(d.path.clone()))
            .padding([6, 12])
            .style(button::secondary),
    ]
    .align_y(Alignment::Center)
    .spacing(10)
    .into()
}

/// Tarjeta de "salud del sistema": qué falta para que DXVK/Vulkan
/// funcionen bien (wine, soporte i386, vulkan-tools, driver mesa).
pub fn system_card(report: &Option<SystemReport>, lang: Lang) -> Element<'_, Message> {
    let t = i18n::tr(lang);
    let body: Element<'_, Message> = match report {
        None => button(text(t.check_system).size(14))
            .on_press(Message::ScanSystemPressed)
            .padding([8, 14])
            .style(button::secondary)
            .into(),
        Some(r) => {
            let item = |label: &str, ok: bool| {
                row![
                    text(if ok { "✓" } else { "✗" })
                        .color(if ok { style::chip_green() } else { style::chip_red() }),
                    text(label.to_string()).size(13),
                ]
                .spacing(6)
            };

            let list = column![
                item(t.wine_label, r.wine),
                item(t.i386_arch_label, r.i386_arch),
                item(t.vulkan_tools_label, r.vulkan_tools),
                item(t.mesa_vulkan_label, r.mesa_vulkan),
            ]
            .spacing(4);

            let action: Element<'_, Message> = if r.all_ok() {
                text(t.system_ready).size(13).color(style::chip_green()).into()
            } else {
                button(text(t.install_missing).size(13))
                    .on_press(Message::InstallSetupPressed)
                    .padding([8, 14])
                    .into()
            };

            column![list, action].spacing(10).into()
        }
    };

    container(
        column![text(t.system_config).size(15), body].spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .style(style::card_style)
    .into()
}
