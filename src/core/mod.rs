/* core/mod.rs
 *
 * Agrupa toda la lógica que no es de interfaz: el estado de la app
 * (app.rs), el manejo de perfiles de Wine (prefix.rs), el chequeo
 * del sistema (system.rs) y la extracción de íconos (icon.rs).
 */

pub mod app;
pub mod icon;
pub mod prefix;
pub mod system;
