/* icon.rs
 *
 * Extrae el ícono real embebido en un .exe de Windows (formato PE)
 * para mostrarlo como miniatura, en vez del avatar genérico con la
 * inicial. Parseamos la estructura binaria a mano en vez de usar
 * una librería externa de terceros: el formato PE está documentado
 * y es estable desde hace décadas (no cambia entre versiones de
 * Windows), así que es un desarrollo controlado, con el mismo
 * criterio que usamos para leer los .lnk.
 *
 * Solo dependemos del crate `image` para el último paso: decodificar
 * los bytes de un .ico ya armado en píxeles RGBA (eso sí conviene
 * una librería madura, en vez de reimplementar un decodificador de
 * BMP/PNG a mano).
 *
 * Si algo falla en cualquier paso (el .exe no tiene ícono, viene en
 * un formato inesperado, etc.), devolvemos None sin generar ruido:
 * quien llama debe usar el avatar de respaldo.
 */

use iced::widget::image::Handle;

const RT_ICON: u32 = 3;
const RT_GROUP_ICON: u32 = 14;

fn u16le(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

struct Section {
    virtual_address: u32,
    virtual_size: u32,
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
}

/// Convierte una dirección virtual relativa (RVA, como aparece
/// cuando el ejecutable está cargado en memoria) en un offset real
/// dentro del archivo en disco, buscando en qué sección cae.
fn rva_to_file_offset(sections: &[Section], rva: u32) -> Option<usize> {
    for s in sections {
        let span = s.virtual_size.max(s.size_of_raw_data);
        if rva >= s.virtual_address && rva < s.virtual_address + span {
            return Some((s.pointer_to_raw_data + (rva - s.virtual_address)) as usize);
        }
    }
    None
}

struct PeInfo {
    sections: Vec<Section>,
    resource_dir_rva: u32,
}

/// Lee los encabezados DOS/PE/COFF/Opcional de un .exe y devuelve la
/// tabla de secciones más la RVA del directorio de recursos
/// (DataDirectory[2]).
fn parse_pe_headers(bytes: &[u8]) -> Option<PeInfo> {
    // Cabecera DOS: firma "MZ" y, en el offset 0x3C, dónde empieza
    // la cabecera PE real.
    if bytes.get(0..2)? != b"MZ" {
        return None;
    }
    let pe_offset = u32le(bytes, 0x3C)? as usize;
    if bytes.get(pe_offset..pe_offset + 4)? != b"PE\0\0" {
        return None;
    }

    // Cabecera COFF (20 bytes) justo después de la firma "PE\0\0".
    let coff = pe_offset + 4;
    let number_of_sections = u16le(bytes, coff + 2)? as usize;
    let size_of_optional_header = u16le(bytes, coff + 16)? as usize;

    // Cabecera opcional: el campo Magic dice si es PE32 (0x10b) o
    // PE32+ / 64 bits (0x20b). El DataDirectory arranca en un
    // offset distinto según cuál sea.
    let optional_header = coff + 20;
    let magic = u16le(bytes, optional_header)?;
    let data_dir_offset = match magic {
        0x10b => optional_header + 96,
        0x20b => optional_header + 112,
        _ => return None,
    };

    // DataDirectory[2] es la tabla de recursos (índice 0=exports,
    // 1=imports, 2=resources). Cada entrada mide 8 bytes.
    let resource_entry_offset = data_dir_offset + 2 * 8;
    let resource_dir_rva = u32le(bytes, resource_entry_offset)?;
    if resource_dir_rva == 0 {
        return None; // el .exe no tiene sección de recursos
    }

    // Tabla de secciones: arranca justo después de la cabecera
    // opcional, una entrada de 40 bytes por sección.
    let section_table_offset = optional_header + size_of_optional_header;
    let mut sections = Vec::with_capacity(number_of_sections);
    for i in 0..number_of_sections {
        let base = section_table_offset + i * 40;
        let virtual_size = u32le(bytes, base + 8)?;
        let virtual_address = u32le(bytes, base + 12)?;
        let size_of_raw_data = u32le(bytes, base + 16)?;
        let pointer_to_raw_data = u32le(bytes, base + 20)?;
        sections.push(Section {
            virtual_address,
            virtual_size,
            size_of_raw_data,
            pointer_to_raw_data,
        });
    }

    Some(PeInfo { sections, resource_dir_rva })
}

/// Una entrada del directorio de recursos: o bien apunta a otro
/// subdirectorio, o bien a los datos finales (hoja).
enum ResourceEntry {
    Directory(usize), // offset de archivo del subdirectorio
    Data(usize),      // offset de archivo del IMAGE_RESOURCE_DATA_ENTRY
}

/// Lee la primera entrada de un directorio de recursos que cumpla
/// `matches` (o la primera de todas si `matches` es None). Los
/// offsets dentro de estas estructuras son relativos a la RVA base
/// del directorio de recursos completo, no al principio del archivo.
fn first_matching_entry(
    bytes: &[u8],
    pe: &PeInfo,
    dir_file_offset: usize,
    matches: Option<u32>,
) -> Option<ResourceEntry> {
    let named = u16le(bytes, dir_file_offset + 12)? as usize;
    let ids = u16le(bytes, dir_file_offset + 14)? as usize;
    let total = named + ids;
    let entries_base = dir_file_offset + 16;

    for i in 0..total {
        let entry_offset = entries_base + i * 8;
        let name_id = u32le(bytes, entry_offset)?;
        let id = name_id & 0x7FFF_FFFF;
        if let Some(want) = matches {
            if id != want {
                continue;
            }
        }
        let data_offset_field = u32le(bytes, entry_offset + 4)?;
        let is_subdirectory = data_offset_field & 0x8000_0000 != 0;
        let relative = data_offset_field & 0x7FFF_FFFF;
        let target_rva = pe.resource_dir_rva + relative;
        let target_file_offset = rva_to_file_offset(&pe.sections, target_rva)?;
        return Some(if is_subdirectory {
            ResourceEntry::Directory(target_file_offset)
        } else {
            ResourceEntry::Data(target_file_offset)
        });
    }
    None
}

/// A partir del offset de archivo de un IMAGE_RESOURCE_DATA_ENTRY,
/// devuelve los bytes crudos del recurso al que apunta.
fn read_data_entry<'a>(bytes: &'a [u8], pe: &PeInfo, data_entry_offset: usize) -> Option<&'a [u8]> {
    let data_rva = u32le(bytes, data_entry_offset)?;
    let size = u32le(bytes, data_entry_offset + 4)? as usize;
    let file_offset = rva_to_file_offset(&pe.sections, data_rva)?;
    bytes.get(file_offset..file_offset + size)
}

/// Baja tres niveles (tipo → nombre → idioma) desde la raíz del
/// directorio de recursos hasta la primera hoja de datos que
/// encuentra para el tipo de recurso `resource_type`, filtrando el
/// nivel "nombre" por `name_id` si se indica.
fn find_first_leaf<'a>(
    bytes: &'a [u8],
    pe: &PeInfo,
    root_offset: usize,
    resource_type: u32,
    name_id: Option<u32>,
) -> Option<&'a [u8]> {
    let type_dir = match first_matching_entry(bytes, pe, root_offset, Some(resource_type))? {
        ResourceEntry::Directory(off) => off,
        ResourceEntry::Data(_) => return None,
    };
    let name_dir = match first_matching_entry(bytes, pe, type_dir, name_id)? {
        ResourceEntry::Directory(off) => off,
        ResourceEntry::Data(_) => return None,
    };
    let lang_leaf = match first_matching_entry(bytes, pe, name_dir, None)? {
        ResourceEntry::Data(off) => off,
        ResourceEntry::Directory(_) => return None,
    };
    read_data_entry(bytes, pe, lang_leaf)
}

/// Una entrada GRPICONDIRENTRY, tal como aparece dentro del recurso
/// RT_GROUP_ICON (no confundir con ICONDIRENTRY de un .ico
/// independiente: acá el último campo es un ID de recurso, no un
/// offset al bitmap).
struct GroupIconEntry {
    width: u8,
    height: u8,
    color_count: u8,
    planes: u16,
    bit_count: u16,
    bytes_in_res: u32,
    id: u16,
}

fn parse_group_icon_dir(bytes: &[u8]) -> Option<Vec<GroupIconEntry>> {
    let count = u16le(bytes, 4)? as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = 6 + i * 14;
        out.push(GroupIconEntry {
            width: *bytes.get(base)?,
            height: *bytes.get(base + 1)?,
            color_count: *bytes.get(base + 2)?,
            planes: u16le(bytes, base + 4)?,
            bit_count: u16le(bytes, base + 6)?,
            bytes_in_res: u32le(bytes, base + 8)?,
            id: u16le(bytes, base + 12)?,
        });
    }
    Some(out)
}

/// De todas las resoluciones disponibles en el grupo de íconos, nos
/// quedamos con la más grande (0 en el byte de tamaño significa
/// "256", igual que en el formato .ico estándar).
fn pick_best<'a>(entries: &'a [GroupIconEntry]) -> Option<&'a GroupIconEntry> {
    let dimension = |v: u8| if v == 0 { 256u32 } else { v as u32 };
    entries.iter().max_by_key(|e| dimension(e.width) * dimension(e.height))
}

/// Arma un archivo .ico independiente y válido a partir de una sola
/// imagen (para poder decodificarlo con el crate `image`, que ya
/// sabe leer ese formato).
fn build_standalone_ico(entry: &GroupIconEntry, image_data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(6 + 16 + image_data.len());
    // ICONDIR
    out.extend_from_slice(&0u16.to_le_bytes()); // Reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // Type = 1 (ícono)
    out.extend_from_slice(&1u16.to_le_bytes()); // Count = 1
    // ICONDIRENTRY
    out.push(entry.width);
    out.push(entry.height);
    out.push(entry.color_count);
    out.push(0); // Reserved
    out.extend_from_slice(&entry.planes.to_le_bytes());
    out.extend_from_slice(&entry.bit_count.to_le_bytes());
    out.extend_from_slice(&(image_data.len() as u32).to_le_bytes()); // BytesInRes
    out.extend_from_slice(&22u32.to_le_bytes()); // ImageOffset: 6 + 16
    let _ = entry.bytes_in_res; // ya usamos el tamaño real de los datos leídos
    out.extend_from_slice(image_data);
    out
}

/// Extrae el ícono principal de un .exe y lo devuelve como un
/// `Handle` de imagen de iced, listo para mostrar. None si el .exe
/// no tiene ícono, no se pudo leer, o vino en un formato que no
/// supimos interpretar — en ese caso, quien llama debe mostrar el
/// avatar de respaldo en vez de esto.
pub fn extract_icon_handle(exe_path: &str) -> Option<Handle> {
    let bytes = std::fs::read(exe_path).ok()?;
    let pe = parse_pe_headers(&bytes)?;
    let root_offset = rva_to_file_offset(&pe.sections, pe.resource_dir_rva)?;

    // 1) El grupo de íconos (RT_GROUP_ICON) nos da la lista de
    //    resoluciones disponibles y a qué ID de RT_ICON apunta cada
    //    una.
    let group_bytes = find_first_leaf(&bytes, &pe, root_offset, RT_GROUP_ICON, None)?;
    let entries = parse_group_icon_dir(group_bytes)?;
    let best = pick_best(&entries)?;

    // 2) Con ese ID, buscamos los bytes reales de la imagen dentro
    //    de RT_ICON.
    let image_data = find_first_leaf(&bytes, &pe, root_offset, RT_ICON, Some(best.id as u32))?;

    // 3) Reconstruimos un .ico válido y lo decodificamos con la
    //    librería `image`, que entiende tanto el formato BMP crudo
    //    como el PNG comprimido que usan los íconos grandes
    //    modernos.
    let ico_bytes = build_standalone_ico(best, image_data);
    let decoded = image::load_from_memory_with_format(&ico_bytes, image::ImageFormat::Ico).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());

    Some(Handle::from_rgba(width, height, rgba.into_raw()))
}
