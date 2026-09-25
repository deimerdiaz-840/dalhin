# Dalhin

Gestor de perfiles de Wine para Linux, con biblioteca de aplicaciones y
chequeo del sistema para DXVK/Vulkan.

![license](https://img.shields.io/badge/license-GPL--3.0-blue)


<table>
<tr>
<td><img width="420" alt="01" src="https://github.com/user-attachments/assets/eed06630-5012-47e0-8166-3765ed3c701f" /></td>
<td><img width="420" alt="03" src="https://github.com/user-attachments/assets/a5a939d9-73e1-43a2-993e-07d9e7badc51" /></td>
</tr>
<tr>
<td><img width="420" alt="02" src="https://github.com/user-attachments/assets/757f6dea-4c61-41f4-8b0e-79de26696ed8" /></td>
<td><img width="420" alt="08" src="https://github.com/user-attachments/assets/8630a4d2-64b1-4dcd-b7dd-51db875ef9b8" /></td>
</tr>
<tr>
<td><img width="420" alt="07" src="https://github.com/user-attachments/assets/9f223524-4692-48e7-9fbd-cf8cc0e71e64" /></td>
<td><img width="420" alt="05" src="https://github.com/user-attachments/assets/cf46f54d-4df4-4fbe-880a-fc5c15f78469" /></td>
</tr>
<tr>
<td><img width="420" alt="04" src="https://github.com/user-attachments/assets/3fa4bda0-d8d0-4da2-b858-93f5d6d2371a" /></td>
<td></td>
</tr>
</table>

## Descargar y ejecutar (binario ya compilado)

La forma más rápida de usar Dalhin es descargar el ejecutable ya compilado,
sin necesidad de instalar Rust ni compilar nada:

1. Andá a la sección [Releases](https://github.com/deimerdiaz-840/dalhin/releases)
   del repositorio y descargá el binario de la última versión.
2. Dale permiso de ejecución al archivo descargado:

   ```bash
   chmod +x dalhin
   ```

3. Ejecutalo:

   ```bash
   ./dalhin
   ```

Al abrirse, Dalhin revisa si tu sistema tiene instalado `wine`, la
arquitectura `i386` (necesaria para apps de 32 bits), `vulkan-tools` y el
driver Vulkan de Mesa (para DXVK). Si te falta algo, te lo indica y te ofrece
un botón para **instalar lo que falta** (te va a pedir la contraseña de
administrador). No hace falta que instales Wine vos mismo de antemano: podés
dejar que la propia aplicación lo instale por vos, o instalarlo manualmente
antes si preferís.

Si preferís instalar todo manualmente antes de abrir la app, en distros
basadas en Debian/Ubuntu podés hacerlo así:

```bash
sudo dpkg --add-architecture i386
sudo apt update
sudo apt install wine wine32 wine64 vulkan-tools mesa-vulkan-drivers
```

En otras distros los paquetes se llaman distinto (por ejemplo `wine-staging`,
`lib32-mesa`, `vulkan-icd-loader` en Arch, o `wine.i686`, `vulkan-tools`,
`mesa-vulkan-drivers` en Fedora), así que revisá el gestor de paquetes de tu
sistema si no usás Debian/Ubuntu.

## Descargar el código fuente

- Desde este repositorio, botón verde **Code → Download ZIP**
  (arriba a la derecha en GitHub), o clonando con git:

  ```bash
  git clone https://github.com/deimerdiaz-840/dalhin.git
  ```

## Compilar desde el código fuente

Necesitás tener instalado [Rust](https://www.rust-lang.org/tools/install) (vía `rustup`).

```bash
cd dalhin
cargo build --release
```

El ejecutable queda en `target/release/dalhin`.

Para correrlo directamente sin compilar antes por separado:

```bash
cargo run --release
```

## Idiomas

La interfaz detecta automáticamente el idioma del sistema (español, inglés,
portugués de Brasil o ruso). También se puede cambiar a mano desde el botón
⚙ en la esquina superior derecha.

## Apoyar el proyecto

Si Dalhin te resultó útil, podés donar desde el botón "☕ Donar" dentro de
la app, o directamente en los links de la sección de sponsors de este
repositorio.

## Licencia

Este proyecto está licenciado bajo la **GNU General Public License v3.0**
(GPL-3.0). Ver el archivo [LICENSE](./LICENSE) para el texto completo.

En resumen: sos libre de usar, estudiar, modificar y redistribuir este
software, pero cualquier trabajo derivado que redistribuyas también tiene
que estar bajo GPL-3.0 y con el código fuente disponible.
