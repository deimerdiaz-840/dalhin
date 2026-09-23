/* system.rs
 *
 * Todo lo que NO es por-prefijo: qué hay instalado en el sistema
 * operativo (wine, soporte de 32 bits, drivers Vulkan) y cómo
 * instalarlo. Esto es la "configuración del PC" — separado de
 * prefix.rs, que es la "configuración de cada Wine".
 */

use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct SystemReport {
    pub wine: bool,
    pub i386_arch: bool,
    pub vulkan_tools: bool,
    pub mesa_vulkan: bool,
}

impl SystemReport {
    pub fn all_ok(&self) -> bool {
        self.wine && self.i386_arch && self.vulkan_tools && self.mesa_vulkan
    }
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Revisa si i386 está habilitado como arquitectura extra de dpkg.
/// Es requisito para instalar wine32 / dxvk-wine32:i386 en un
/// sistema de 64 bits basado en Debian/Ubuntu.
fn i386_enabled() -> bool {
    std::process::Command::new("dpkg")
        .arg("--print-foreign-architectures")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("i386"))
        .unwrap_or(false)
}

/// Revisa si el sistema tiene un driver Vulkan de Mesa cargado,
/// consultando el ICD (Installable Client Driver) de mesa que
/// vulkan-tools/mesa-vulkan-drivers instalan en /usr/share/vulkan.
fn mesa_vulkan_present() -> bool {
    std::path::Path::new("/usr/share/vulkan/icd.d")
        .read_dir()
        .map(|mut entries| {
            entries.any(|e| {
                e.ok()
                    .map(|e| e.file_name().to_string_lossy().contains("mesa"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Escanea el sistema (no bloqueante, todo son lecturas rápidas de
/// disco/`which`) y devuelve qué falta para correr juegos/apps con
/// buen rendimiento vía DXVK.
pub fn scan_system() -> SystemReport {
    SystemReport {
        wine: command_exists("wine"),
        i386_arch: i386_enabled(),
        vulkan_tools: command_exists("vulkaninfo"),
        mesa_vulkan: mesa_vulkan_present(),
    }
}

/// Comandos que hacen falta para dejar el sistema listo, en el
/// orden correcto (i386 antes que los paquetes que lo necesitan).
/// Se muestran al usuario, nunca se ejecutan sin que él lo pida.
pub fn missing_setup_commands(report: &SystemReport) -> Vec<String> {
    let mut cmds = Vec::new();

    if !report.i386_arch {
        cmds.push("dpkg --add-architecture i386".to_string());
    }

    let mut pkgs = Vec::new();
    if !report.wine {
        pkgs.push("wine");
        pkgs.push("wine32");
        pkgs.push("wine64");
    }
    if !report.vulkan_tools {
        pkgs.push("vulkan-tools");
    }
    if !report.mesa_vulkan {
        pkgs.push("mesa-vulkan-drivers");
    }
    if !pkgs.is_empty() {
        cmds.push("apt update".to_string());
        cmds.push(format!("apt install -y {}", pkgs.join(" ")));
    }

    cmds
}

/// Corre los comandos de setup elevando privilegios con pkexec (abre
/// el diálogo gráfico de contraseña de polkit — no requiere que la
/// app misma corra como root ni que el usuario tenga sudo por
/// terminal). Todo se ejecuta como un único `bash -c` para que
/// pkexec pida la contraseña una sola vez.
pub async fn run_setup(commands: Vec<String>) -> Result<String, String> {
    if commands.is_empty() {
        return Ok("Nada que instalar, el sistema ya está listo".to_string());
    }
    let script = commands.join(" && ");

    let output = Command::new("pkexec")
        .arg("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .await
        .map_err(|e| format!("No se pudo lanzar pkexec: {e}. ¿Está instalado polkit?"))?;

    if output.status.success() {
        Ok("Configuración del sistema completada".to_string())
    } else {
        Err(format!(
            "Falló la instalación: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
