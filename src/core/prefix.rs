/* prefix.rs
 *
 * Gestión de prefijos de Wine: crear, listar y persistir en disco.
 * Sin base de datos, sin ORM — un JSON simple en
 * ~/.local/share/dalhin/prefixes.json.
 */

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;

/// Detecta si estamos corriendo dentro del sandbox de Flatpak
/// (existe /.flatpak-info dentro del sandbox).
fn running_in_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// Construye el Command base para invocar `wine`. Si estamos en el
/// sandbox de Flatpak, usa `flatpak-spawn --host` para llegar al wine
/// instalado en el sistema real. Fuera de Flatpak (caso normal ahora
/// que dejamos Builder), simplemente invoca `wine` directo.
fn wine_command() -> Command {
    if running_in_flatpak() {
        let mut cmd = Command::new("flatpak-spawn");
        cmd.arg("--host");
        cmd
    } else {
        Command::new("wine")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryApp {
    pub name: String,
    pub exe_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefix {
    pub name: String,
    pub path: String,
    pub arch: String, // "win32" o "win64"
    #[serde(default)]
    pub category: String, // "aplicaciones" o "juegos"
    /// true si el prefix fue detectado/registrado desde una carpeta
    /// que ya existía (p. ej. ~/.wine), en vez de creado por dalhin.
    /// Los externos nunca se borran del disco al "eliminar" — solo
    /// se quita la entrada del JSON.
    #[serde(default)]
    pub external: bool,
    /// Ruta al instalador (setup.exe) que se lanza con el botón
    /// "Instalar". None hasta que el usuario elige uno. Se conserva
    /// después de instalar, por si hay que reinstalar o modificar
    /// algo más adelante.
    #[serde(default, alias = "game_exe")]
    pub setup_exe: Option<String>,
    /// Ruta al .exe del juego/launcher ya instalado, que se lanza
    /// con el botón "Jugar". None hasta que el usuario lo elige.
    #[serde(default)]
    pub launcher_exe: Option<String>,
    /// Biblioteca de aplicaciones/juegos agregados a este prefijo.
    /// Un mismo prefijo puede tener varias apps instaladas adentro
    /// (p. ej. varios juegos de GOG bajo la misma carpeta
    /// "GOG Games"), cada una con su propio botón para jugar.
    #[serde(default)]
    pub library: Vec<LibraryApp>,
}

fn data_dir() -> PathBuf {
    let mut dir = dirs::data_dir().expect("No se pudo encontrar el directorio de datos");
    dir.push("dalhin");
    dir
}

fn db_path() -> PathBuf {
    let mut p = data_dir();
    p.push("prefixes.json");
    p
}

fn settings_path() -> PathBuf {
    let mut p = data_dir();
    p.push("settings.json");
    p
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Settings {
    /// Última carpeta desde la que el usuario eligió un instalador,
    /// para que el selector abra ahí la próxima vez en vez de
    /// arrancar siempre en la carpeta de inicio (los instaladores
    /// suelen vivir todos en el mismo lugar, p. ej. ~/Descargas).
    #[serde(default)]
    last_installer_dir: Option<String>,
}

fn load_settings() -> Settings {
    let path = settings_path();
    if !path.exists() {
        return Settings::default();
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

fn save_settings(settings: &Settings) {
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(settings_path(), json);
    }
}

/// Guarda la carpeta que contiene `exe_path` como la última usada
/// para elegir instaladores.
fn remember_installer_dir(exe_path: &str) {
    if let Some(parent) = std::path::Path::new(exe_path).parent() {
        let mut settings = load_settings();
        settings.last_installer_dir = Some(parent.to_string_lossy().to_string());
        save_settings(&settings);
    }
}

/// Carpeta donde viven los prefijos en sí (los WINEPREFIX reales).
fn prefixes_root() -> PathBuf {
    let mut p = data_dir();
    p.push("prefixes");
    p
}

pub fn list_prefixes() -> Vec<Prefix> {
    let path = db_path();
    if !path.exists() {
        return Vec::new();
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

fn save_prefixes(prefixes: &[Prefix]) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())?;
    let json = serde_json::to_string_pretty(prefixes).unwrap();
    std::fs::write(db_path(), json)
}

/// Valida que un nombre de prefix sea seguro para usarse como
/// componente de una ruta. Rechaza separadores de directorio, ".."
/// y nombres que empiecen con "." (ocultos / relativos), para que
/// nadie pueda escribir fuera de prefixes_root() vía path traversal
/// (p. ej. un nombre como "../../.config").
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
        && !name.starts_with('.')
}

/// Crea un prefijo nuevo: registra la entrada en el JSON y lanza
/// `wine wineboot` para inicializar el WINEPREFIX en disco.
/// Devuelve el Prefix creado o un mensaje de error legible.
pub async fn create_prefix(name: String, arch: String, category: String) -> Result<Prefix, String> {
    if name.trim().is_empty() {
        return Err("El nombre no puede estar vacío".to_string());
    }
    if !is_safe_name(&name) {
        return Err("El nombre no puede contener '/', '\\\\', ni empezar con '.'".to_string());
    }

    let mut prefixes = list_prefixes();
    if prefixes.iter().any(|p| p.name == name) {
        return Err(format!("Ya existe un prefijo llamado «{name}»"));
    }

    let mut prefix_path = prefixes_root();
    prefix_path.push(&name);
    std::fs::create_dir_all(&prefix_path)
        .map_err(|e| format!("No se pudo crear la carpeta: {e}"))?;

    let mut cmd = wine_command();
    if running_in_flatpak() {
        cmd.arg(format!("--env=WINEPREFIX={}", prefix_path.display()));
        cmd.arg(format!("--env=WINEARCH={arch}"));
        cmd.arg("wine");
        cmd.arg("wineboot");
    } else {
        cmd.env("WINEPREFIX", &prefix_path);
        cmd.env("WINEARCH", &arch);
        cmd.arg("wineboot");
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("No se pudo ejecutar wine: {e}. ¿Está instalado?"))?;

    if !output.status.success() {
        return Err(format!(
            "wineboot falló: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let prefix = Prefix {
        name: name.clone(),
        path: prefix_path.to_string_lossy().to_string(),
        arch,
        category,
        external: false,
        setup_exe: None,
        launcher_exe: None,
        library: Vec::new(),
    };

    prefixes.push(prefix.clone());
    save_prefixes(&prefixes).map_err(|e| format!("No se pudo guardar: {e}"))?;

    Ok(prefix)
}

/// Abre winecfg apuntando al WINEPREFIX de este prefijo, para que el
/// usuario configure versión de Windows, DLLs, audio, etc. de forma
/// aislada por prefijo (nunca toca ningún otro prefix).
pub async fn open_winecfg(prefix: Prefix) -> Result<(), String> {
    let mut cmd = if running_in_flatpak() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host");
        c.arg(format!("--env=WINEPREFIX={}", prefix.path));
        c.arg("winecfg");
        c
    } else {
        let mut c = Command::new("winecfg");
        c.env("WINEPREFIX", &prefix.path);
        c
    };

    cmd.spawn()
        .map_err(|e| format!("No se pudo abrir winecfg: {e}. ¿Está instalado wine?"))?;
    Ok(())
}

/// Abre el selector de archivos nativo del sistema (portal de
/// GTK/Qt vía rfd) filtrado a .exe, empezando en `start_dir` si se
/// indica una carpeta y existe. None si el usuario cancela.
pub async fn pick_exe(start_dir: Option<PathBuf>) -> Option<String> {
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter("Ejecutables de Windows", &["exe"])
        .set_title("Elegir instalador o juego (.exe)");
    if let Some(dir) = start_dir {
        if dir.is_dir() {
            dialog = dialog.set_directory(dir);
        }
    }
    let file = dialog.pick_file().await?;
    Some(file.path().to_string_lossy().to_string())
}

/// Selector para elegir el instalador: abre en la última carpeta
/// desde la que se eligió uno (si existe), y al elegir uno nuevo
/// recuerda esa carpeta para la próxima vez.
pub async fn pick_setup_exe() -> Option<String> {
    let start_dir = load_settings()
        .last_installer_dir
        .map(PathBuf::from)
        .filter(|p| p.is_dir());
    let picked = pick_exe(start_dir).await?;
    remember_installer_dir(&picked);
    Some(picked)
}

/// Selector para elegir el .exe del juego ya instalado: la ruta
/// real varía muchísimo de un juego a otro (p. ej.
/// `GOG Games/Life is Strange/Binaries/Win32/...`, o `Win64` en
/// otros), así que no podemos adivinar la carpeta exacta. Como
/// mínimo, arrancamos el selector en `drive_c` del prefijo en vez
/// de la carpeta de inicio del usuario, para ahorrar la mitad del
/// camino.
pub async fn pick_launcher_exe(prefix: &Prefix) -> Option<String> {
    let mut drive_c = PathBuf::from(&prefix.path);
    drive_c.push("drive_c");
    let start_dir = drive_c.is_dir().then_some(drive_c);
    pick_exe(start_dir).await
}

/// Corre un .exe dentro de un prefijo existente. Se usa tanto para
/// el instalador como para el juego/launcher: en ambos casos solo
/// lanzamos el proceso (spawn) y no esperamos a que termine, porque
/// muchos instaladores de Windows arrancan una ventana hija y el
/// proceso original vuelve enseguida — esperar ahí adivina mal
/// cuándo terminaste, así que mejor no adivinar nada.
pub async fn run_exe(prefix: Prefix, exe_path: String) -> Result<(), String> {
    let mut cmd = wine_command();
    if running_in_flatpak() {
        cmd.arg(format!("--env=WINEPREFIX={}", prefix.path));
        cmd.arg("wine");
        cmd.arg(&exe_path);
    } else {
        cmd.env("WINEPREFIX", &prefix.path);
        cmd.arg(&exe_path);
    }
    cmd.spawn()
        .map_err(|e| format!("No se pudo lanzar el programa: {e}"))?;
    Ok(())
}

/// Nombres de archivo que casi siempre son artefactos del instalador
/// y no el juego en sí (desinstaladores, redistribuibles, runtimes,
/// manejadores de errores de motores como Unity/Unreal). Se comparan
/// en minúsculas y por "contiene", no por igualdad exacta, porque
/// suelen venir con sufijos de versión. Incluye los mismos patrones
/// que usa Bottles en su lista `ignored_patterns` de
/// `Manager.get_programs()`.
const NOT_A_GAME_HINTS: &[&str] = &[
    "unins", "uninstall", "setup", "install", "redist", "vcredist",
    "dxsetup", "directx", "dotnetfx", "dotnet-", "oalinst", "vc_redist",
    "vcruntime", "ucrtbase", "crashpad", "crashreport", "crashhandler",
    "updater", "update.exe", "debug", "report", "err", "website",
    "web site", "user_manual",
];

fn looks_like_installer_artifact(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    NOT_A_GAME_HINTS.iter().any(|hint| lower.contains(hint))
}

/// Carpetas que no vale la pena recorrer buscando el juego: el
/// propio Windows simulado por Wine y la papelera.
fn is_skippable_dir(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    lower == "windows" || lower == "$recycle.bin" || lower == "recycler"
}

/// Recorre recursivamente `dir` buscando archivos con la extensión
/// `ext` (sin punto, p. ej. "exe" o "lnk") modificados después de
/// `since`. Los junta en `results` como (ruta, fecha de
/// modificación). No filtra por nombre acá — eso lo decide quien
/// llama, según qué está buscando.
fn scan_for_new_files(
    dir: &std::path::Path,
    since: std::time::SystemTime,
    depth: u32,
    ext: &str,
    results: &mut Vec<(String, std::time::SystemTime)>,
) {
    if depth > 8 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if is_skippable_dir(&file_name) {
                continue;
            }
            scan_for_new_files(&path, since, depth + 1, ext, results);
            continue;
        }

        let matches_ext = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false);
        if !matches_ext {
            continue;
        }

        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified >= since {
                    results.push((path.to_string_lossy().to_string(), modified));
                }
            }
        }
    }
}

fn read_u32_le(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes(slice.try_into().unwrap()))
}

fn read_u16_le(bytes: &[u8], at: usize) -> Option<u16> {
    let slice = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes(slice.try_into().unwrap()))
}

/// Lee el campo LocalBasePath de la sección LinkInfo del formato
/// binario Shell Link (.lnk), yendo directo a los offsets exactos
/// en vez de adivinar por texto suelto. Es el mismo algoritmo que
/// usa Bottles en `LnkUtils.get_data()` (GPLv3, Mirko Brombin),
/// que a su vez viene de <https://gist.github.com/Winand/997ed38269e899eb561991a0c663fa49>.
/// Es mucho más confiable que buscar strings sueltas.
fn parse_lnk_binary(bytes: &[u8]) -> Option<String> {
    // Saltar los primeros 20 bytes (HeaderSize + LinkCLSID) y leer
    // la estructura LinkFlags (4 bytes) en 0x14.
    let lflags = read_u32_le(bytes, 0x14)?;
    let mut position: usize = 0x18;

    if lflags & 0x01 == 1 {
        // HasLinkTargetIDList: saltar esa estructura, cuyo tamaño
        // está guardado justo antes (en 0x4C) del propio IDList.
        let id_list_size = read_u16_le(bytes, 0x4C)? as usize;
        position = id_list_size + 0x4E;
    }

    let last_pos = position;
    position += 0x04;

    // LinkInfoSize: cuánto mide toda la sección LinkInfo.
    let length = read_u32_le(bytes, last_pos)? as usize;

    // Saltar LinkInfoHeaderSize + LinkInfoFlags + VolumeIDOffset (12 bytes).
    position += 0x0C;

    // LocalBasePathOffset, relativo al inicio de LinkInfo.
    let lbpos = read_u32_le(bytes, position)? as usize;
    position = last_pos + lbpos;

    let end = (length + last_pos).checked_sub(0x02)?;
    if position >= end || end > bytes.len() {
        return None;
    }

    let slice = &bytes[position..end];
    if let Ok(s) = std::str::from_utf8(slice) {
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    // Respaldo por si viniera en UTF-16 (poco común para este campo,
    // pero por las dudas).
    let utf16: Vec<u16> = slice.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16(&utf16).ok().filter(|s| !s.is_empty())
}

/// Extrae, de un acceso directo .lnk de Windows, la ruta del
/// programa al que apunta. Primero intenta leer la estructura
/// binaria real (LocalBasePath dentro de LinkInfo — ver
/// `parse_lnk_binary`); si por lo que sea no se puede (.lnk con
/// formato inesperado), recurre a buscar cadenas de texto legibles
/// que terminen en ".exe" como respaldo.
fn extract_exe_path_from_lnk(lnk_path: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(lnk_path).ok()?;

    if let Some(path) = parse_lnk_binary(&bytes) {
        if path.to_lowercase().ends_with(".exe") {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();
    collect_ascii_strings(&bytes, &mut candidates);
    collect_utf16le_strings(&bytes, &mut candidates);

    candidates
        .into_iter()
        .filter(|s| s.to_lowercase().ends_with(".exe") && s.contains('\\'))
        .max_by_key(|s| s.len())
}

fn collect_ascii_strings(bytes: &[u8], out: &mut Vec<String>) {
    let mut current = Vec::new();
    for &b in bytes {
        if b.is_ascii_graphic() || b == b' ' {
            current.push(b);
        } else {
            if current.len() >= 5 {
                if let Ok(s) = String::from_utf8(current.clone()) {
                    out.push(s);
                }
            }
            current.clear();
        }
    }
    if current.len() >= 5 {
        if let Ok(s) = String::from_utf8(current) {
            out.push(s);
        }
    }
}

fn collect_utf16le_strings(bytes: &[u8], out: &mut Vec<String>) {
    let mut current: Vec<u16> = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let unit = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        let printable = (0x20..0x7f).contains(&unit);
        if printable {
            current.push(unit);
        } else {
            if current.len() >= 5 {
                if let Ok(s) = String::from_utf16(&current) {
                    out.push(s);
                }
            }
            current.clear();
        }
        i += 2;
    }
    if current.len() >= 5 {
        if let Ok(s) = String::from_utf16(&current) {
            out.push(s);
        }
    }
}

/// Busca `name` dentro de `dir` sin importar mayúsculas/minúsculas
/// (Windows no distingue, pero el filesystem de Linux sí).
fn find_case_insensitive(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    let target = name.to_lowercase();
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().to_lowercase() == target {
            return Some(entry.path());
        }
    }
    None
}

/// Convierte la ruta cruda que encontramos dentro del .lnk (con
/// letra de unidad estilo Windows, p. ej. `C:\GOG Games\Bread &
/// Fred Demo\Bread&Fred.exe`) en una ruta real dentro de
/// `drive_c`, y confirma que el archivo exista de verdad antes de
/// darla por buena. Prueba primero como ruta absoluta desde la
/// unidad; si no existe, prueba relativa a la carpeta donde está el
/// propio .lnk (algunos accesos directos guardan solo el nombre o
/// una ruta relativa).
fn resolve_lnk_target(drive_c: &std::path::Path, lnk_path: &std::path::Path, raw: &str) -> Option<PathBuf> {
    let normalized = raw.replace('\\', "/");

    // Caso 1: ruta con letra de unidad ("C:/...").
    if normalized.len() > 2 && normalized.as_bytes()[1] == b':' {
        let rest = normalized[2..].trim_start_matches('/');
        if let Some(found) = resolve_relative(drive_c, rest) {
            return Some(found);
        }
    }

    // Caso 2: ruta relativa a la carpeta donde vive el propio .lnk.
    if let Some(parent) = lnk_path.parent() {
        if let Some(found) = resolve_relative(parent, &normalized) {
            return Some(found);
        }
        // Última opción: solo el nombre de archivo, en la misma carpeta.
        if let Some(file_name) = std::path::Path::new(&normalized).file_name() {
            let candidate = parent.join(file_name);
            if candidate.is_file() {
                return Some(candidate);
            }
            if let Some(found) = find_case_insensitive(parent, &file_name.to_string_lossy()) {
                return Some(found);
            }
        }
    }

    None
}

/// Intenta unir `base` con la ruta relativa `rest` (con '/' como
/// separador), tolerando diferencias de mayúsculas/minúsculas
/// segmento por segmento, ya que Windows no distingue y el .lnk
/// puede no coincidir exactamente con los nombres reales en disco.
fn resolve_relative(base: &std::path::Path, rest: &str) -> Option<PathBuf> {
    let mut current = base.to_path_buf();
    for segment in rest.split('/').filter(|s| !s.is_empty()) {
        let direct = current.join(segment);
        if direct.exists() {
            current = direct;
            continue;
        }
        current = find_case_insensitive(&current, segment)?;
    }
    if current.is_file() {
        Some(current)
    } else {
        None
    }
}

/// Ubicaciones típicas donde Windows deja los accesos directos de
/// los programas instalados. Son las mismas rutas que usa Bottles
/// en `Manager.get_programs()`: no hace falta adivinar por fecha de
/// modificación si sabemos exactamente dónde mirar.
fn shortcut_search_roots(drive_c: &std::path::Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(entries) = std::fs::read_dir(drive_c.join("users")) {
        for entry in entries.flatten() {
            let user_dir = entry.path();
            if !user_dir.is_dir() {
                continue;
            }
            roots.push(user_dir.join("Desktop"));
            roots.push(user_dir.join("Start Menu").join("Programs"));
            roots.push(
                user_dir
                    .join("AppData")
                    .join("Roaming")
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs"),
            );
        }
    }
    roots.push(
        drive_c
            .join("ProgramData")
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs"),
    );
    roots
}

/// Busca las aplicaciones/juegos instalados en un prefijo.
///
/// Estrategia, de más a menos confiable:
/// 1. Buscar accesos directos (.lnk) en el Escritorio y el Menú
///    Inicio (todas las ubicaciones típicas de Windows) y leer a
///    qué .exe apuntan. Es la señal más confiable posible: la deja
///    el propio instalador, sin importar cómo se llame el juego, la
///    carpeta o el ejecutable — funciona igual para cualquier
///    prefijo futuro. No depende de fechas, así que sirve tanto
///    recién instalado como para prefijos externos que ya tenían
///    juegos de antes.
/// 2. Si no aparece ningún acceso directo útil (algunos instaladores
///    portátiles no crean ninguno), recurrir al .exe modificado más
///    recientemente después de `since_for_fallback` que no parezca
///    parte del instalador.
///
/// Devuelve una lista de (nombre sugerido, ruta del .exe) — puede
/// haber más de uno, por ejemplo si el prefijo tiene varios juegos
/// instalados (como una carpeta "GOG Games" con varios títulos).
pub async fn list_installed_programs(
    prefix: Prefix,
    since_for_fallback: std::time::SystemTime,
) -> Result<Vec<(String, String)>, String> {
    let mut drive_c = PathBuf::from(&prefix.path);
    drive_c.push("drive_c");
    if !drive_c.exists() {
        return Err("No se encontró la carpeta drive_c del prefijo".to_string());
    }

    let mut lnk_files: Vec<(String, std::time::SystemTime)> = Vec::new();
    for root in shortcut_search_roots(&drive_c) {
        if root.is_dir() {
            scan_for_new_files(&root, std::time::UNIX_EPOCH, 0, "lnk", &mut lnk_files);
        }
    }

    let mut seen = std::collections::HashSet::new();
    let mut programs: Vec<(String, String)> = Vec::new();
    for (lnk_path_str, _) in &lnk_files {
        let lnk_path = std::path::Path::new(lnk_path_str);
        let Some(raw) = extract_exe_path_from_lnk(lnk_path) else {
            continue;
        };
        let Some(resolved) = resolve_lnk_target(&drive_c, lnk_path, &raw) else {
            continue;
        };
        let resolved_str = resolved.to_string_lossy().to_string();
        let file_name = resolved
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        if looks_like_installer_artifact(&file_name) {
            continue;
        }
        if seen.insert(resolved_str.clone()) {
            programs.push((derive_app_name(&resolved_str), resolved_str));
        }
    }

    // Respaldo: sin accesos directos, probamos con el .exe nuevo más
    // reciente que no parezca parte del instalador.
    if programs.is_empty() {
        let mut exe_candidates = Vec::new();
        scan_for_new_files(&drive_c, since_for_fallback, 0, "exe", &mut exe_candidates);
        exe_candidates.retain(|(path, _)| {
            let file_name = std::path::Path::new(path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            !looks_like_installer_artifact(&file_name)
        });
        exe_candidates.sort_by(|a, b| b.1.cmp(&a.1));
        if let Some((path, _)) = exe_candidates.into_iter().next() {
            programs.push((derive_app_name(&path), path));
        }
    }

    Ok(programs)
}

#[derive(Debug, Clone)]
pub struct DetectedPrefix {
    pub name: String,
    pub path: String,
    pub arch: String,
}

/// Lee la cabecera de system.reg para saber si el prefix es win32 o
/// win64. Wine escribe una línea "#arch=winXX" cerca del principio
/// del archivo. Si no se encuentra, se asume win64 por ser lo más
/// común hoy en día.
fn detect_arch(prefix_path: &std::path::Path) -> String {
    let system_reg = prefix_path.join("system.reg");
    if let Ok(content) = std::fs::read_to_string(&system_reg) {
        for line in content.lines().take(20) {
            if let Some(arch) = line.strip_prefix("#arch=") {
                return arch.trim().to_string();
            }
        }
    }
    "win64".to_string()
}

/// Busca prefijos de Wine "sueltos" en el home (~/.wine, ~/.wine-*,
/// etc.) que no pasaron por dalhin. Un WINEPREFIX válido se reconoce
/// porque tiene system.reg en su raíz — esa es la única señal que
/// usamos, no adivinamos por el nombre de la carpeta.
pub fn detect_existing_prefixes() -> Vec<DetectedPrefix> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let registered_paths: Vec<String> = list_prefixes().into_iter().map(|p| p.path).collect();
    let mut found = Vec::new();

    let entries = match std::fs::read_dir(&home) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if !file_name.starts_with(".wine") || !path.is_dir() {
            continue;
        }
        if !path.join("system.reg").exists() {
            continue; // carpeta .wine* pero no es un prefix inicializado
        }

        let path_str = path.to_string_lossy().to_string();
        if registered_paths.contains(&path_str) {
            continue; // ya está en el JSON, no lo repetimos
        }

        found.push(DetectedPrefix {
            name: file_name.trim_start_matches('.').to_string(),
            arch: detect_arch(&path),
            path: path_str,
        });
    }

    found
}

/// Registra en el JSON un prefix que ya existe en disco (detectado o
/// agregado a mano). A diferencia de create_prefix, no corre
/// wineboot ni crea la carpeta: el prefix ya está inicializado.
pub fn register_prefix(
    name: String,
    path: String,
    arch: String,
    category: String,
) -> Result<Prefix, String> {
    if !is_safe_name(&name) {
        return Err("El nombre no puede contener '/', '\\\\', ni empezar con '.'".to_string());
    }
    let mut prefixes = list_prefixes();
    if prefixes.iter().any(|p| p.name == name || p.path == path) {
        return Err(format!("«{name}» ya está registrado"));
    }

    let prefix = Prefix {
        name,
        path,
        arch,
        category,
        external: true,
        setup_exe: None,
        launcher_exe: None,
        library: Vec::new(),
    };
    prefixes.push(prefix.clone());
    save_prefixes(&prefixes).map_err(|e| format!("No se pudo guardar: {e}"))?;
    Ok(prefix)
}

/// Asocia el instalador (setup.exe) a un prefix ya existente, para
/// que el botón "Instalar" lo recuerde la próxima vez (por ejemplo,
/// para reinstalar o correr el instalador de nuevo más adelante).
pub fn set_setup_exe(name: &str, exe_path: String) -> Result<Prefix, String> {
    let mut prefixes = list_prefixes();
    let prefix = prefixes
        .iter_mut()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("No se encontró el prefijo «{name}»"))?;
    prefix.setup_exe = Some(exe_path);
    let updated = prefix.clone();
    save_prefixes(&prefixes).map_err(|e| format!("No se pudo guardar: {e}"))?;
    Ok(updated)
}

/// Asocia el .exe del juego/launcher ya instalado a un prefix ya
/// existente, para que el botón "Jugar" lo recuerde la próxima vez
/// en lugar de volver a correr el instalador.
pub fn set_launcher_exe(name: &str, exe_path: String) -> Result<Prefix, String> {
    let mut prefixes = list_prefixes();
    let prefix = prefixes
        .iter_mut()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("No se encontró el prefijo «{name}»"))?;
    prefix.launcher_exe = Some(exe_path);
    let updated = prefix.clone();
    save_prefixes(&prefixes).map_err(|e| format!("No se pudo guardar: {e}"))?;
    Ok(updated)
}

/// Nombres de carpeta demasiado genéricos como para servir de
/// nombre de la app (arquitectura, carpeta de binarios, etc.) — se
/// saltan al adivinar el nombre a partir de la ruta del .exe.
const GENERIC_FOLDER_NAMES: &[&str] = &[
    "win32", "win64", "x86", "x64", "binaries", "bin", "game", "games", "data", "system",
];

/// Adivina un nombre legible para la app a partir de la ruta de su
/// .exe: sube por las carpetas contenedoras buscando la primera que
/// no sea un nombre genérico de arquitectura/binarios (p. ej. en
/// ".../Life is Strange/Binaries/Win32/juego.exe" da "Life is
/// Strange"). Si no encuentra ninguna, usa el nombre del .exe sin
/// extensión.
fn derive_app_name(exe_path: &str) -> String {
    let path = std::path::Path::new(exe_path);
    for ancestor in path.ancestors().skip(1) {
        if let Some(os_name) = ancestor.file_name() {
            let name = os_name.to_string_lossy().to_string();
            if !name.is_empty() && !GENERIC_FOLDER_NAMES.contains(&name.to_lowercase().as_str()) {
                return name;
            }
        }
    }
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Aplicación".to_string())
}

/// Agrega una app a la biblioteca del prefijo. Si ya existe una
/// entrada con la misma ruta, no la duplica. El nombre se adivina
/// automáticamente a partir de la carpeta del .exe.
pub fn add_library_app(name: &str, exe_path: String) -> Result<Prefix, String> {
    let mut prefixes = list_prefixes();
    let prefix = prefixes
        .iter_mut()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("No se encontró el prefijo «{name}»"))?;

    if !prefix.library.iter().any(|app| app.exe_path == exe_path) {
        prefix.library.push(LibraryApp {
            name: derive_app_name(&exe_path),
            exe_path,
        });
    }
    let updated = prefix.clone();
    save_prefixes(&prefixes).map_err(|e| format!("No se pudo guardar: {e}"))?;
    Ok(updated)
}

/// Quita una app de la biblioteca del prefijo por su ruta de .exe.
pub fn remove_library_app(name: &str, exe_path: &str) -> Result<Prefix, String> {
    let mut prefixes = list_prefixes();
    let prefix = prefixes
        .iter_mut()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("No se encontró el prefijo «{name}»"))?;
    prefix.library.retain(|app| app.exe_path != exe_path);
    let updated = prefix.clone();
    save_prefixes(&prefixes).map_err(|e| format!("No se pudo guardar: {e}"))?;
    Ok(updated)
}

/// Elimina un prefijo. Si es un prefix creado por dalhin, borra
/// también su carpeta en disco. Si es externo (detectado, p. ej.
/// ~/.wine), solo se quita la entrada del JSON — nunca se toca la
/// carpeta real, porque no es de dalhin y puede tener datos del
/// usuario que no vinimos a gestionar.
pub fn delete_prefix(name: &str) -> Result<(), String> {
    let prefixes = list_prefixes();
    let prefix = prefixes
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("No se encontró el prefijo «{name}»"))?;

    if !prefix.external {
        std::fs::remove_dir_all(&prefix.path)
            .map_err(|e| format!("No se pudo borrar la carpeta: {e}"))?;
    }

    let remaining: Vec<Prefix> = prefixes.into_iter().filter(|p| p.name != name).collect();
    save_prefixes(&remaining).map_err(|e| format!("No se pudo guardar: {e}"))
}
