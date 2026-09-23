/* app.rs
 *
 * Estado de la aplicación (Model), mensajes (Message) y las dos
 * funciones que pide iced: update() y view(). La lógica de negocio
 * real vive en prefix.rs y system.rs — aquí solo se orquesta.
 */

use crate::i18n::{self, Lang};
use crate::core::prefix::{self, DetectedPrefix, Prefix};
use crate::core::system::{self, SystemReport};
use crate::ui::components;
use crate::ui::style::StatusKind;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::{column, container, scrollable, text, Column};
use iced::{Element, Length, Task, Theme};
use std::collections::HashMap;
use std::time::SystemTime;

/// Qué pestaña está activa: "Prefijos" (gestor de prefijos de Wine)
/// o "Biblioteca" (todas las apps agregadas). Nunca se muestran ni
/// se usan las dos al mismo tiempo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Prefixes,
    Library,
    Settings,
}

pub struct Dalhin {
    prefixes: Vec<Prefix>,
    new_name: String,
    status: Option<(StatusKind, String)>,
    detected: Vec<DetectedPrefix>,
    system_report: Option<SystemReport>,
    /// Prefijos donde se apretó "Instalar" en esta sesión y todavía
    /// no tienen launcher: para esos mostramos el botón "Ya instalé,
    /// buscar apps" en vez del selector manual. Guarda el momento
    /// en que se lanzó el instalador, como respaldo por si no
    /// aparece ningún acceso directo (ahí sí se filtra por fecha).
    installing_since: HashMap<String, SystemTime>,
    active_tab: Tab,
    /// Íconos reales ya extraídos de cada .exe, para no volver a
    /// leer y parsear el mismo archivo cada vez que se redibuja la
    /// pantalla. Se llena de a poco, en segundo plano, con
    /// `queue_missing_icon_loads`. `None` significa "se intentó y no
    /// se encontró ningún ícono" (o todavía se está cargando) — en
    /// ambos casos la UI muestra el avatar de respaldo.
    icon_cache: HashMap<String, Option<ImageHandle>>,
    /// Idioma de la interfaz, detectado una sola vez al arrancar a
    /// partir del idioma del sistema (ver `i18n::Lang::detect`), y
    /// modificable a mano desde la pestaña de Configuración (⚙).
    lang: Lang,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    CreatePressed,
    PrefixCreated(Result<Prefix, String>),
    DeletePressed(String),
    DetectPressed,
    AddDetectedPressed(String),
    WinecfgPressed(String),
    WinecfgOpened(Result<(), String>),
    GameLaunched(Result<(), String>),
    InstallPressed(String),
    ChangeSetupPressed(String),
    SetupExeChosen(String, Option<String>),
    ScanProgramsPressed(String),
    ProgramsFound(String, Result<Vec<(String, String)>, String>),
    AddAppPressed(String),
    AppExeChosen(String, Option<String>),
    LaunchAppPressed(String, String),
    RemoveAppPressed(String, String),
    ScanSystemPressed,
    SystemScanned(SystemReport),
    InstallSetupPressed,
    SetupFinished(Result<String, String>),
    TabSelected(Tab),
    IconLoaded(String, Option<ImageHandle>),
    LangSelected(Lang),
    DonatePressed,
}

impl Dalhin {
    pub fn new() -> (Self, Task<Message>) {
        let mut app = Self {
            prefixes: prefix::list_prefixes(),
            new_name: String::new(),
            status: None,
            detected: Vec::new(),
            system_report: None,
            installing_since: HashMap::new(),
            active_tab: Tab::Prefixes,
            icon_cache: HashMap::new(),
            lang: Lang::detect(),
        };
        let icon_task = app.queue_missing_icon_loads();
        (app, icon_task)
    }

    /// Encola la extracción de ícono real para cada .exe de la
    /// biblioteca que todavía no la tenga (ni esté ya en curso).
    /// Cada extracción se hace en su propia tarea async, así una
    /// biblioteca grande no traba la interfaz mientras se procesan.
    fn queue_missing_icon_loads(&mut self) -> Task<Message> {
        let mut paths: Vec<String> = Vec::new();
        for p in &self.prefixes {
            for app in &p.library {
                paths.push(app.exe_path.clone());
            }
            if let Some(exe) = &p.launcher_exe {
                paths.push(exe.clone());
            }
        }
        paths.sort();
        paths.dedup();
        paths.retain(|path| !self.icon_cache.contains_key(path));

        if paths.is_empty() {
            return Task::none();
        }

        // Se marcan como "en curso" (None) para no volver a
        // encolarlas mientras se resuelven.
        for path in &paths {
            self.icon_cache.insert(path.clone(), None);
        }

        let tasks = paths.into_iter().map(|path| {
            let key = path.clone();
            Task::perform(async move { crate::core::icon::extract_icon_handle(&path) }, move |handle| {
                Message::IconLoaded(key.clone(), handle)
            })
        });
        Task::batch(tasks)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NameChanged(value) => {
                self.new_name = value;
                Task::none()
            }
            Message::CreatePressed => {
                let name = self.new_name.trim().to_string();
                if name.is_empty() {
                    self.status = Some((StatusKind::Err, i18n::msg_name_empty(self.lang)));
                    return Task::none();
                }
                self.status = Some((StatusKind::Info, i18n::msg_creating(self.lang, &name)));
                Task::perform(
                    prefix::create_prefix(name, "win64".into(), "aplicaciones".into()),
                    Message::PrefixCreated,
                )
            }
            Message::PrefixCreated(result) => {
                match result {
                    Ok(p) => {
                        self.status = Some((StatusKind::Ok, i18n::msg_profile_created(self.lang, &p.name)));
                        self.new_name.clear();
                        self.prefixes = prefix::list_prefixes();
                    }
                    Err(e) => self.status = Some((StatusKind::Err, e)),
                }
                Task::none()
            }
            Message::DeletePressed(name) => {
                match prefix::delete_prefix(&name) {
                    Ok(()) => {
                        self.status = Some((StatusKind::Ok, i18n::msg_profile_deleted(self.lang, &name)));
                        self.prefixes = prefix::list_prefixes();
                        self.installing_since.remove(&name);
                    }
                    Err(e) => self.status = Some((StatusKind::Err, e)),
                }
                Task::none()
            }
            Message::DetectPressed => {
                self.detected = prefix::detect_existing_prefixes();
                if self.detected.is_empty() {
                    self.status = Some((StatusKind::Info, i18n::msg_no_new_profiles_found(self.lang)));
                }
                Task::none()
            }
            Message::AddDetectedPressed(path) => {
                if let Some(d) = self.detected.iter().find(|d| d.path == path).cloned() {
                    match prefix::register_prefix(d.name.clone(), d.path.clone(), d.arch.clone(), "aplicaciones".into()) {
                        Ok(p) => {
                            self.status = Some((StatusKind::Ok, i18n::msg_added(self.lang, &p.name)));
                            self.prefixes = prefix::list_prefixes();
                            self.detected.retain(|x| x.path != path);
                        }
                        Err(e) => self.status = Some((StatusKind::Err, e)),
                    }
                }
                Task::none()
            }
            Message::WinecfgPressed(name) => {
                if let Some(p) = self.prefixes.iter().find(|p| p.name == name).cloned() {
                    self.status = Some((StatusKind::Info, i18n::msg_opening_winecfg(self.lang, &name)));
                    return Task::perform(prefix::open_winecfg(p), Message::WinecfgOpened);
                }
                Task::none()
            }
            Message::WinecfgOpened(result) => {
                if let Err(e) = result {
                    self.status = Some((StatusKind::Err, e));
                }
                Task::none()
            }
            Message::GameLaunched(result) => {
                match result {
                    Ok(()) => self.status = Some((StatusKind::Ok, i18n::msg_program_started(self.lang))),
                    Err(e) => self.status = Some((StatusKind::Err, e)),
                }
                Task::none()
            }
            Message::InstallPressed(name) => {
                // Solo llega acá si el botón estaba habilitado, es
                // decir, si ya hay un instalador agregado (ver
                // components::prefix_row). Correrlo es una acción
                // aparte de "agregar instalador": así no se vuelve a
                // ejecutar por accidente ni se confunde con elegir el
                // archivo.
                if let Some(p) = self.prefixes.iter().find(|p| p.name == name).cloned() {
                    if let Some(exe) = p.setup_exe.clone() {
                        self.installing_since.insert(name.clone(), SystemTime::now());
                        self.status = Some((
                            StatusKind::Info,
                            i18n::msg_running_installer(self.lang, &name),
                        ));
                        return Task::perform(prefix::run_exe(p, exe), Message::GameLaunched);
                    }
                }
                Task::none()
            }
            Message::ChangeSetupPressed(name) => Task::perform(prefix::pick_setup_exe(), move |path| {
                Message::SetupExeChosen(name.clone(), path)
            }),
            Message::SetupExeChosen(name, Some(path)) => match prefix::set_setup_exe(&name, path) {
                Ok(_) => {
                    self.prefixes = prefix::list_prefixes();
                    self.status = Some((
                        StatusKind::Ok,
                        i18n::msg_installer_added(self.lang, &name),
                    ));
                    Task::none()
                }
                Err(e) => {
                    self.status = Some((StatusKind::Err, e));
                    Task::none()
                }
            },
            Message::SetupExeChosen(_, None) => Task::none(), // el usuario canceló el selector
            Message::ScanProgramsPressed(name) => {
                if let Some(p) = self.prefixes.iter().find(|p| p.name == name).cloned() {
                    let since = self
                        .installing_since
                        .get(&name)
                        .copied()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    self.status = Some((
                        StatusKind::Info,
                        i18n::msg_scanning_apps(self.lang, &name),
                    ));
                    return Task::perform(prefix::list_installed_programs(p, since), move |r| {
                        Message::ProgramsFound(name.clone(), r)
                    });
                }
                Task::none()
            }
            Message::ProgramsFound(name, result) => {
                match result {
                    Ok(programs) if !programs.is_empty() => {
                        let mut added = 0usize;
                        for (_, exe_path) in &programs {
                            if prefix::add_library_app(&name, exe_path.clone()).is_ok() {
                                added += 1;
                            }
                        }
                        self.prefixes = prefix::list_prefixes();
                        self.installing_since.remove(&name);
                        // Si la tarjeta todavía no tenía un juego
                        // principal para el botón "Jugar" rápido, usamos
                        // el primero que encontramos.
                        let needs_launcher = self
                            .prefixes
                            .iter()
                            .find(|p| p.name == name)
                            .map(|p| p.launcher_exe.is_none())
                            .unwrap_or(false);
                        if needs_launcher {
                            if let Some((_, first_exe)) = programs.first() {
                                let _ = prefix::set_launcher_exe(&name, first_exe.clone());
                                self.prefixes = prefix::list_prefixes();
                            }
                        }
                        self.status = Some((
                            StatusKind::Ok,
                            i18n::msg_apps_found(self.lang, added, &name),
                        ));
                        return self.queue_missing_icon_loads();
                    }
                    Ok(_) => {
                        self.status = Some((
                            StatusKind::Err,
                            i18n::msg_no_apps_found(self.lang, &name),
                        ));
                    }
                    Err(e) => self.status = Some((StatusKind::Err, e)),
                }
                Task::none()
            }
            Message::AddAppPressed(name) => {
                if let Some(p) = self.prefixes.iter().find(|p| p.name == name).cloned() {
                    return Task::perform(
                        async move { prefix::pick_launcher_exe(&p).await },
                        move |path| Message::AppExeChosen(name.clone(), path),
                    );
                }
                Task::none()
            }
            Message::AppExeChosen(name, Some(path)) => match prefix::add_library_app(&name, path) {
                Ok(_) => {
                    self.prefixes = prefix::list_prefixes();
                    self.status = Some((StatusKind::Ok, i18n::msg_app_added_to_library(self.lang)));
                    self.queue_missing_icon_loads()
                }
                Err(e) => {
                    self.status = Some((StatusKind::Err, e));
                    Task::none()
                }
            },
            Message::AppExeChosen(_, None) => Task::none(), // el usuario canceló el selector
            Message::IconLoaded(exe_path, handle) => {
                self.icon_cache.insert(exe_path, handle);
                Task::none()
            }
            Message::LaunchAppPressed(name, exe_path) => {
                if let Some(p) = self.prefixes.iter().find(|p| p.name == name).cloned() {
                    self.status = Some((StatusKind::Info, i18n::msg_starting(self.lang)));
                    return Task::perform(prefix::run_exe(p, exe_path), Message::GameLaunched);
                }
                Task::none()
            }
            Message::RemoveAppPressed(name, exe_path) => {
                match prefix::remove_library_app(&name, &exe_path) {
                    Ok(_) => {
                        self.prefixes = prefix::list_prefixes();
                        self.status = Some((StatusKind::Ok, i18n::msg_app_removed_from_library(self.lang)));
                    }
                    Err(e) => self.status = Some((StatusKind::Err, e)),
                }
                Task::none()
            }
            Message::ScanSystemPressed => {
                Task::perform(async { system::scan_system() }, Message::SystemScanned)
            }
            Message::SystemScanned(report) => {
                self.system_report = Some(report);
                Task::none()
            }
            Message::InstallSetupPressed => {
                if let Some(report) = &self.system_report {
                    let cmds = system::missing_setup_commands(report);
                    self.status = Some((StatusKind::Info, i18n::msg_installing_setup(self.lang)));
                    return Task::perform(system::run_setup(cmds), Message::SetupFinished);
                }
                Task::none()
            }
            Message::SetupFinished(result) => {
                match result {
                    Ok(msg) => {
                        self.status = Some((StatusKind::Ok, msg));
                        self.system_report = Some(system::scan_system());
                    }
                    Err(e) => self.status = Some((StatusKind::Err, e)),
                }
                Task::none()
            }
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                Task::none()
            }
            Message::LangSelected(lang) => {
                self.lang = lang;
                Task::none()
            }
            Message::DonatePressed => {
                // Reemplazá esta URL por tu página real de donaciones
                // (GitHub Sponsors, Ko-fi, PayPal.me, etc.) cuando la
                // tengas decidida.
                let _ = open::that("https://github.com/sponsors/TU_USUARIO");
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::{button, pick_list, row, text_input};

        let t = i18n::tr(self.lang);

        let header = column![
            text(t.app_title).size(28),
            text(t.subtitle).size(14).color(crate::ui::style::muted_color()),
        ]
        .spacing(4);

        // Botón de configuración (⚙): lleva a la pantalla de
        // configuración (sistema + idioma), igual que un botón de
        // pestaña más, pero fuera de la fila de pestañas.
        let settings_button = button(text("⚙").size(20))
            .on_press(Message::TabSelected(Tab::Settings))
            .padding(8)
            .style(if self.active_tab == Tab::Settings { button::primary } else { button::text });

        let donate_button = button(text(t.donate).size(13))
            .on_press(Message::DonatePressed)
            .padding([8, 14])
            .style(button::secondary);

        let header_row = row![header, iced::widget::horizontal_space(), donate_button, settings_button]
            .spacing(8)
            .align_y(iced::Alignment::Start);

        // Pestañas: "Perfiles" y "Biblioteca" son mutuamente
        // excluyentes, nunca se ven ni se usan las dos a la vez.
        let tab_button = |label: &str, tab: Tab| {
            let is_active = self.active_tab == tab;
            button(text(label.to_string()).size(14))
                .on_press(Message::TabSelected(tab))
                .padding([8, 16])
                .style(if is_active { button::primary } else { button::secondary })
        };
        let tabs = row![
            tab_button(t.tab_profiles_manager, Tab::Prefixes),
            tab_button(t.tab_library, Tab::Library),
        ]
        .spacing(8);

        let status = components::status_banner(&self.status);

        let body: Element<'_, Message> = match self.active_tab {
            Tab::Prefixes => {
                let create_card = container(
                    column![
                        text(t.new_profile).size(15),
                        row![
                            text_input(t.profile_name_placeholder, &self.new_name)
                                .on_input(Message::NameChanged)
                                .on_submit(Message::CreatePressed)
                                .padding(10),
                            button(text(t.create).size(14)).on_press(Message::CreatePressed).padding([10, 18]),
                        ]
                        .spacing(10)
                        .align_y(iced::Alignment::Center),
                    ]
                    .spacing(10),
                )
                .padding(16)
                .width(Length::Fill)
                .style(crate::ui::style::card_style);

                let detect_row = row![
                    button(text(t.detect_existing_profiles).size(14))
                        .on_press(Message::DetectPressed)
                        .padding([8, 14])
                        .style(button::secondary),
                ];

                let detected_section: Element<'_, Message> = if self.detected.is_empty() {
                    column![].into()
                } else {
                    let items: Column<Message> = self.detected.iter().fold(column![].spacing(8), |col, d| {
                        col.push(components::detected_row(d, self.lang))
                    });
                    container(
                        column![
                            text(i18n::msg_detected_unregistered(self.lang, self.detected.len()))
                                .size(14)
                                .color(crate::ui::style::muted_color()),
                            items,
                        ]
                        .spacing(10),
                    )
                    .padding(16)
                    .width(Length::Fill)
                    .style(crate::ui::style::card_style)
                    .into()
                };

                let prefix_list: Element<'_, Message> = if self.prefixes.is_empty() {
                    container(
                        text(t.no_profiles_yet)
                            .size(14)
                            .color(crate::ui::style::muted_color()),
                    )
                    .padding(24)
                    .width(Length::Fill)
                    .style(crate::ui::style::card_style)
                    .into()
                } else {
                    let items: Column<Message> = self.prefixes.iter().fold(column![].spacing(8), |col, p| {
                        col.push(components::prefix_row(
                            p,
                            self.installing_since.contains_key(&p.name),
                            &self.icon_cache,
                            self.lang,
                        ))
                    });
                    items.into()
                };

                column![
                    create_card,
                    detect_row,
                    detected_section,
                    text(i18n::msg_profiles_count(self.lang, self.prefixes.len())).size(16),
                    prefix_list,
                ]
                .spacing(18)
                .into()
            }
            Tab::Library => components::library_view(&self.prefixes, &self.icon_cache, self.lang),
            Tab::Settings => {
                let language_card = container(
                    column![
                        text(t.language).size(15),
                        pick_list(Lang::all().to_vec(), Some(self.lang), Message::LangSelected).padding(8),
                    ]
                    .spacing(10),
                )
                .padding(16)
                .width(Length::Fill)
                .style(crate::ui::style::card_style);

                column![
                    components::system_card(&self.system_report, self.lang),
                    language_card,
                ]
                .spacing(18)
                .into()
            }
        };

        let content = column![header_row, tabs, status, body]
            .spacing(18)
            .padding(24)
            .max_width(720);

        container(scrollable(container(content).center_x(Length::Fill)).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .into()
    }
}

pub fn theme(_state: &Dalhin) -> Theme {
    Theme::Dark
}
