/* i18n.rs
 *
 * Idioma de la interfaz. Se detecta una sola vez, al arrancar, leyendo
 * las variables de entorno estándar de Linux (LC_ALL, LC_MESSAGES,
 * LANGUAGE, LANG, en ese orden de prioridad). No hay selector manual
 * a propósito: si el sistema está en otro idioma que no soportamos
 * todavía, caemos en inglés.
 *
 * Los textos fijos de la interfaz viven en `Tr` (uno por idioma, vía
 * `tr(lang)`). Los mensajes de estado que llevan datos variables
 * (nombre de perfil, cantidades, etc.) son funciones `msg_*` que arman
 * el `String` final para evitar parsear plantillas en tiempo de
 * ejecución.
 */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Es,
    En,
    PtBr,
    Ru,
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl Lang {
    /// Todos los idiomas soportados, en el orden en que se muestran
    /// en el selector de idioma.
    pub const fn all() -> [Lang; 4] {
        [Lang::Es, Lang::En, Lang::PtBr, Lang::Ru]
    }

    /// Nombre del idioma tal como se muestra en su propio selector
    /// (cada uno escrito en su propio idioma, como es costumbre en
    /// selectores de idioma).
    pub const fn label(&self) -> &'static str {
        match self {
            Lang::Es => "Español",
            Lang::En => "English",
            Lang::PtBr => "Português (BR)",
            Lang::Ru => "Русский",
        }
    }

    /// Detecta el idioma a partir de las variables de entorno del
    /// sistema. Si no se reconoce ninguna, cae en inglés (es el
    /// idioma "universal" más seguro para un idioma no soportado
    /// todavía, en vez de asumir español).
    pub fn detect() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"] {
            if let Ok(raw) = std::env::var(var) {
                if let Some(lang) = Lang::from_locale_str(&raw) {
                    return lang;
                }
            }
        }
        Lang::En
    }

    /// `LANGUAGE` puede traer una lista tipo "pt_BR:en_US"; nos
    /// quedamos con el primer idioma que reconozcamos de la lista.
    fn from_locale_str(raw: &str) -> Option<Lang> {
        for candidate in raw.split(':') {
            let lower = candidate.trim().to_lowercase();
            if lower.starts_with("ru") {
                return Some(Lang::Ru);
            }
            if lower.starts_with("pt") {
                // Por ahora solo tenemos portugués de Brasil: para
                // cualquier variante de "pt" es la traducción más
                // cercana que tenemos disponible.
                return Some(Lang::PtBr);
            }
            if lower.starts_with("es") {
                return Some(Lang::Es);
            }
            if lower.starts_with("en") {
                return Some(Lang::En);
            }
        }
        None
    }
}

/// Textos fijos de la interfaz (sin datos variables).
pub struct Tr {
    pub app_title: &'static str,
    pub subtitle: &'static str,
    pub tab_profiles_manager: &'static str,
    pub tab_library: &'static str,
    pub system_config: &'static str,
    pub check_system: &'static str,
    pub new_profile: &'static str,
    pub profile_name_placeholder: &'static str,
    pub create: &'static str,
    pub detect_existing_profiles: &'static str,
    pub no_profiles_yet: &'static str,
    pub external: &'static str,
    pub change_installer: &'static str,
    pub add_installer: &'static str,
    pub install: &'static str,
    pub winecfg: &'static str,
    pub delete: &'static str,
    pub open: &'static str,
    pub remove: &'static str,
    pub no_apps_yet: &'static str,
    pub installing_hint: &'static str,
    pub scan_installed_apps: &'static str,
    pub scan_done_button: &'static str,
    pub add_to_library: &'static str,
    pub library_empty: &'static str,
    pub add: &'static str,
    pub wine_label: &'static str,
    pub i386_arch_label: &'static str,
    pub vulkan_tools_label: &'static str,
    pub mesa_vulkan_label: &'static str,
    pub system_ready: &'static str,
    pub install_missing: &'static str,
    pub language: &'static str,
    pub donate: &'static str,
}

pub fn tr(lang: Lang) -> Tr {
    match lang {
        Lang::Es => Tr {
            app_title: "Dalhin",
            subtitle: "Gestor de perfiles de Wine",
            tab_profiles_manager: "Gestor de perfiles",
            tab_library: "Biblioteca",
            system_config: "Configuración del sistema",
            check_system: "Revisar sistema",
            new_profile: "Nuevo perfil",
            profile_name_placeholder: "Nombre del perfil",
            create: "Crear",
            detect_existing_profiles: "Detectar perfiles existentes",
            no_profiles_yet: "No tienes perfiles todavía. Crea uno arriba o detecta los que ya tengas.",
            external: "externo",
            change_installer: "Cambiar instalador",
            add_installer: "Agregar instalador",
            install: "Instalar",
            winecfg: "winecfg",
            delete: "Eliminar",
            open: "Abrir",
            remove: "Quitar",
            no_apps_yet: "Todavía no hay apps agregadas en este perfil.",
            installing_hint: "Instalando… cuando termines, buscá las apps instaladas con el botón de abajo.",
            scan_installed_apps: "Buscar aplicaciones instaladas",
            scan_done_button: "Ya instalé, buscar apps",
            add_to_library: "+ Agregar a biblioteca",
            library_empty: "Todavía no agregaste ninguna app. Andá a la pestaña «Gestor de perfiles» y usá «+ Agregar a biblioteca» en el perfil que quieras.",
            add: "Agregar",
            wine_label: "wine",
            i386_arch_label: "arquitectura i386 (para apps de 32 bits)",
            vulkan_tools_label: "vulkan-tools",
            mesa_vulkan_label: "driver Vulkan de Mesa",
            system_ready: "Sistema listo para DXVK/Vulkan",
            install_missing: "Instalar lo que falta (pide contraseña)",
            language: "Idioma",
            donate: "☕ Donar",
        },
        Lang::En => Tr {
            app_title: "Dalhin",
            subtitle: "Wine profile manager",
            tab_profiles_manager: "Profile manager",
            tab_library: "Library",
            system_config: "System configuration",
            check_system: "Check system",
            new_profile: "New profile",
            profile_name_placeholder: "Profile name",
            create: "Create",
            detect_existing_profiles: "Detect existing profiles",
            no_profiles_yet: "You don't have any profiles yet. Create one above or detect ones you already have.",
            external: "external",
            change_installer: "Change installer",
            add_installer: "Add installer",
            install: "Install",
            winecfg: "winecfg",
            delete: "Delete",
            open: "Open",
            remove: "Remove",
            no_apps_yet: "No apps added to this profile yet.",
            installing_hint: "Installing… when you're done, look for installed apps with the button below.",
            scan_installed_apps: "Scan for installed apps",
            scan_done_button: "Already installed, scan apps",
            add_to_library: "+ Add to library",
            library_empty: "You haven't added any apps yet. Go to the “Profile manager” tab and use “+ Add to library” on the profile you want.",
            add: "Add",
            wine_label: "wine",
            i386_arch_label: "i386 architecture (for 32-bit apps)",
            vulkan_tools_label: "vulkan-tools",
            mesa_vulkan_label: "Mesa Vulkan driver",
            system_ready: "System ready for DXVK/Vulkan",
            install_missing: "Install what's missing (will ask for password)",
            language: "Language",
            donate: "☕ Donate",
        },
        Lang::PtBr => Tr {
            app_title: "Dalhin",
            subtitle: "Gerenciador de perfis do Wine",
            tab_profiles_manager: "Gerenciador de perfis",
            tab_library: "Biblioteca",
            system_config: "Configuração do sistema",
            check_system: "Verificar sistema",
            new_profile: "Novo perfil",
            profile_name_placeholder: "Nome do perfil",
            create: "Criar",
            detect_existing_profiles: "Detectar perfis existentes",
            no_profiles_yet: "Você ainda não tem perfis. Crie um acima ou detecte os que já existem.",
            external: "externo",
            change_installer: "Trocar instalador",
            add_installer: "Adicionar instalador",
            install: "Instalar",
            winecfg: "winecfg",
            delete: "Excluir",
            open: "Abrir",
            remove: "Remover",
            no_apps_yet: "Ainda não há apps adicionados a este perfil.",
            installing_hint: "Instalando… quando terminar, procure os apps instalados com o botão abaixo.",
            scan_installed_apps: "Procurar aplicativos instalados",
            scan_done_button: "Já instalei, procurar apps",
            add_to_library: "+ Adicionar à biblioteca",
            library_empty: "Você ainda não adicionou nenhum app. Vá até a aba “Gerenciador de perfis” e use “+ Adicionar à biblioteca” no perfil desejado.",
            add: "Adicionar",
            wine_label: "wine",
            i386_arch_label: "arquitetura i386 (para apps de 32 bits)",
            vulkan_tools_label: "vulkan-tools",
            mesa_vulkan_label: "driver Vulkan da Mesa",
            system_ready: "Sistema pronto para DXVK/Vulkan",
            install_missing: "Instalar o que falta (vai pedir senha)",
            language: "Idioma",
            donate: "☕ Doar",
        },
        Lang::Ru => Tr {
            app_title: "Dalhin",
            subtitle: "Менеджер профилей Wine",
            tab_profiles_manager: "Профили",
            tab_library: "Библиотека",
            system_config: "Настройка системы",
            check_system: "Проверить систему",
            new_profile: "Новый профиль",
            profile_name_placeholder: "Название профиля",
            create: "Создать",
            detect_existing_profiles: "Найти существующие профили",
            no_profiles_yet: "У вас пока нет профилей. Создайте новый выше или найдите уже существующие.",
            external: "внешний",
            change_installer: "Сменить установщик",
            add_installer: "Добавить установщик",
            install: "Установить",
            winecfg: "winecfg",
            delete: "Удалить",
            open: "Открыть",
            remove: "Убрать",
            no_apps_yet: "В этом профиле пока нет приложений.",
            installing_hint: "Идёт установка… когда закончите, найдите установленные приложения кнопкой ниже.",
            scan_installed_apps: "Найти установленные приложения",
            scan_done_button: "Установка завершена, искать приложения",
            add_to_library: "+ Добавить в библиотеку",
            library_empty: "Вы ещё не добавили ни одного приложения. Перейдите на вкладку «Профили» и используйте «+ Добавить в библиотеку» в нужном профиле.",
            add: "Добавить",
            wine_label: "wine",
            i386_arch_label: "архитектура i386 (для 32-битных приложений)",
            vulkan_tools_label: "vulkan-tools",
            mesa_vulkan_label: "драйвер Vulkan от Mesa",
            system_ready: "Система готова для DXVK/Vulkan",
            install_missing: "Установить недостающее (запросит пароль)",
            language: "Язык",
            donate: "☕ Поддержать",
        },
    }
}

// --- Mensajes de estado con datos variables --------------------------------

pub fn msg_name_empty(lang: Lang) -> String {
    match lang {
        Lang::Es => "El nombre no puede estar vacío".into(),
        Lang::En => "Name can't be empty".into(),
        Lang::PtBr => "O nome não pode ficar vazio".into(),
        Lang::Ru => "Имя не может быть пустым".into(),
    }
}

pub fn msg_creating(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!("Creando «{name}»…"),
        Lang::En => format!("Creating “{name}”…"),
        Lang::PtBr => format!("Criando “{name}”…"),
        Lang::Ru => format!("Создание «{name}»…"),
    }
}

pub fn msg_profile_created(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!("Perfil «{name}» creado"),
        Lang::En => format!("Profile “{name}” created"),
        Lang::PtBr => format!("Perfil “{name}” criado"),
        Lang::Ru => format!("Профиль «{name}» создан"),
    }
}

pub fn msg_profile_deleted(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!("Perfil «{name}» eliminado"),
        Lang::En => format!("Profile “{name}” deleted"),
        Lang::PtBr => format!("Perfil “{name}” excluído"),
        Lang::Ru => format!("Профиль «{name}» удалён"),
    }
}

pub fn msg_no_new_profiles_found(lang: Lang) -> String {
    match lang {
        Lang::Es => "No se encontraron perfiles sueltos nuevos".into(),
        Lang::En => "No new loose profiles were found".into(),
        Lang::PtBr => "Nenhum perfil novo foi encontrado".into(),
        Lang::Ru => "Новых профилей не найдено".into(),
    }
}

pub fn msg_added(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!("«{name}» agregado"),
        Lang::En => format!("“{name}” added"),
        Lang::PtBr => format!("“{name}” adicionado"),
        Lang::Ru => format!("«{name}» добавлен"),
    }
}

pub fn msg_opening_winecfg(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!("Abriendo winecfg de «{name}»…"),
        Lang::En => format!("Opening winecfg for “{name}”…"),
        Lang::PtBr => format!("Abrindo o winecfg de “{name}”…"),
        Lang::Ru => format!("Открываю winecfg для «{name}»…"),
    }
}

pub fn msg_program_started(lang: Lang) -> String {
    match lang {
        Lang::Es => "Programa iniciado".into(),
        Lang::En => "Program started".into(),
        Lang::PtBr => "Programa iniciado".into(),
        Lang::Ru => "Программа запущена".into(),
    }
}

pub fn msg_running_installer(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!(
            "Ejecutando instalador de «{name}»… cuando termines, apretá «Ya instalé, buscar apps»."
        ),
        Lang::En => format!(
            "Running the installer for “{name}”… when you're done, click “Already installed, scan apps”."
        ),
        Lang::PtBr => format!(
            "Executando o instalador de “{name}”… quando terminar, clique em “Já instalei, procurar apps”."
        ),
        Lang::Ru => format!(
            "Запускаю установщик «{name}»… когда закончите, нажмите «Установка завершена, искать приложения»."
        ),
    }
}

pub fn msg_installer_added(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!("Instalador agregado a «{name}». Ya podés apretar «Instalar»."),
        Lang::En => format!("Installer added to “{name}”. You can now click “Install”."),
        Lang::PtBr => format!("Instalador adicionado a “{name}”. Agora você pode clicar em “Instalar”."),
        Lang::Ru => format!("Установщик добавлен в «{name}». Теперь можно нажать «Установить»."),
    }
}

pub fn msg_scanning_apps(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!("Buscando aplicaciones instaladas en «{name}»…"),
        Lang::En => format!("Scanning for installed apps in “{name}”…"),
        Lang::PtBr => format!("Procurando aplicativos instalados em “{name}”…"),
        Lang::Ru => format!("Ищу установленные приложения в «{name}»…"),
    }
}

pub fn msg_apps_found(lang: Lang, added: usize, name: &str) -> String {
    match lang {
        Lang::Es => {
            let plural = if added == 1 { "aplicación" } else { "aplicaciones" };
            format!("Encontré {added} {plural} en «{name}» y las agregué a la biblioteca.")
        }
        Lang::En => {
            let plural = if added == 1 { "app" } else { "apps" };
            format!("Found {added} {plural} in “{name}” and added them to the library.")
        }
        Lang::PtBr => {
            let plural = if added == 1 { "aplicativo" } else { "aplicativos" };
            format!("Encontrei {added} {plural} em “{name}” e adicionei à biblioteca.")
        }
        Lang::Ru => {
            let plural = if added == 1 { "приложение" } else { "приложений" };
            format!("Найдено {added} {plural} в «{name}», добавлено в библиотеку.")
        }
    }
}

pub fn msg_no_apps_found(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Es => format!(
            "No encontré ninguna aplicación instalada en «{name}». Probá «Elegir a mano» para señalarla vos."
        ),
        Lang::En => format!(
            "No installed apps were found in “{name}”. Try picking it manually instead."
        ),
        Lang::PtBr => format!(
            "Nenhum aplicativo instalado foi encontrado em “{name}”. Tente escolher manualmente."
        ),
        Lang::Ru => format!(
            "В «{name}» не найдено установленных приложений. Попробуйте выбрать вручную."
        ),
    }
}

pub fn msg_app_added_to_library(lang: Lang) -> String {
    match lang {
        Lang::Es => "Aplicación agregada a la biblioteca".into(),
        Lang::En => "App added to the library".into(),
        Lang::PtBr => "Aplicativo adicionado à biblioteca".into(),
        Lang::Ru => "Приложение добавлено в библиотеку".into(),
    }
}

pub fn msg_starting(lang: Lang) -> String {
    match lang {
        Lang::Es => "Iniciando…".into(),
        Lang::En => "Starting…".into(),
        Lang::PtBr => "Iniciando…".into(),
        Lang::Ru => "Запуск…".into(),
    }
}

pub fn msg_app_removed_from_library(lang: Lang) -> String {
    match lang {
        Lang::Es => "Aplicación quitada de la biblioteca".into(),
        Lang::En => "App removed from the library".into(),
        Lang::PtBr => "Aplicativo removido da biblioteca".into(),
        Lang::Ru => "Приложение удалено из библиотеки".into(),
    }
}

pub fn msg_installing_setup(lang: Lang) -> String {
    match lang {
        Lang::Es => "Instalando… puede pedir tu contraseña".into(),
        Lang::En => "Installing… it may ask for your password".into(),
        Lang::PtBr => "Instalando… pode pedir sua senha".into(),
        Lang::Ru => "Установка… может запросить пароль".into(),
    }
}

pub fn msg_installer_label(lang: Lang, file: &str) -> String {
    match lang {
        Lang::Es => format!("Instalador: {file}"),
        Lang::En => format!("Installer: {file}"),
        Lang::PtBr => format!("Instalador: {file}"),
        Lang::Ru => format!("Установщик: {file}"),
    }
}

pub fn msg_detected_unregistered(lang: Lang, count: usize) -> String {
    match lang {
        Lang::Es => format!("Encontrados sin registrar ({count})"),
        Lang::En => format!("Found but not registered ({count})"),
        Lang::PtBr => format!("Encontrados e não registrados ({count})"),
        Lang::Ru => format!("Найдено незарегистрированных ({count})"),
    }
}

pub fn msg_profiles_count(lang: Lang, count: usize) -> String {
    match lang {
        Lang::Es => format!("Perfiles ({count})"),
        Lang::En => format!("Profiles ({count})"),
        Lang::PtBr => format!("Perfis ({count})"),
        Lang::Ru => format!("Профили ({count})"),
    }
}

pub fn msg_library_count(lang: Lang, count: usize) -> String {
    match lang {
        Lang::Es => format!("Biblioteca ({count})"),
        Lang::En => format!("Library ({count})"),
        Lang::PtBr => format!("Biblioteca ({count})"),
        Lang::Ru => format!("Библиотека ({count})"),
    }
}
