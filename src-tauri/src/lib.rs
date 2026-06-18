use base64::{engine::general_purpose, Engine as _};
use exif::{In, Reader, Tag};
use image::ImageFormat;
use mailparse::{addrparse_header, MailHeaderMap, ParsedMail};
use serde::Serialize;
use std::{
    env, fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::Command,
    time::UNIX_EPOCH,
};
use tauri::Manager;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentPayload {
    path: Option<String>,
    file_name: String,
    kind: DocumentKind,
    title: String,
    source: String,
    markdown: Option<String>,
    email: Option<EmailPayload>,
    image: Option<ImagePayload>,
    media: Option<MediaPayload>,
    text: Option<TextPayload>,
    info: FileInfoPayload,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum DocumentKind {
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "audio")]
    Audio,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "docx")]
    Docx,
    #[serde(rename = "unsupported")]
    Unsupported,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmailPayload {
    headers: Vec<EmailHeader>,
    from: Option<String>,
    to: Option<String>,
    subject: Option<String>,
    date: Option<String>,
    body_html: Option<String>,
    body_text: Option<String>,
}

#[derive(Debug, Serialize)]
struct EmailHeader {
    name: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImagePayload {
    data_url: String,
    mime_type: String,
    render_note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MediaPayload {
    data_url: String,
    mime_type: String,
    render_note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextPayload {
    text: String,
    language: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileInfoPayload {
    general: Vec<InfoEntry>,
    sections: Vec<InfoSection>,
}

#[derive(Debug, Serialize)]
struct InfoSection {
    title: String,
    entries: Vec<InfoEntry>,
}

#[derive(Debug, Serialize)]
struct InfoEntry {
    label: String,
    value: String,
}

#[tauri::command]
fn get_initial_file() -> Option<String> {
    env::args().skip(1).find(|arg| {
        let trimmed = arg.trim();
        !trimmed.is_empty() && !trimmed.starts_with('-') && Path::new(trimmed).is_file()
    })
}

#[tauri::command]
fn choose_file() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("Open in Rads Quick Viewer")
        .add_filter("Supported files", &supported_extensions())
        .add_filter("Markdown", &["md", "markdown"])
        .add_filter("Email", &["eml"])
        .add_filter(
            "Images",
            &[
                "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "ico", "svg", "avif",
                "heic", "heif",
            ],
        )
        .add_filter("Audio", &["mp3", "wav", "ogg", "oga", "flac", "m4a", "aac", "opus"])
        .add_filter("Video", &["mp4", "m4v", "webm", "ogv", "mov", "avi", "mkv"])
        .add_filter(
            "Text",
            &[
                "txt", "text", "log", "csv", "tsv", "json", "xml", "yaml", "yml", "toml", "ini",
                "html", "htm", "css", "js", "jsx", "ts", "tsx", "rs", "py", "php", "rb", "go",
                "java", "c", "cpp", "h", "hpp", "cs", "sql",
            ],
        )
        .add_filter("Word document", &["docx"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_sibling_file(path: String, direction: i32) -> Result<Option<String>, String> {
    let current = Path::new(&path);
    let directory = match current.parent() {
        Some(directory) => directory,
        None => return Ok(None),
    };
    let current_name = match current.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => return Ok(None),
    };

    let mut files = fs::read_dir(directory)
        .map_err(|error| format!("Unable to read directory: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| supported_extensions().contains(&ext.to_ascii_lowercase().as_str()))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    files.sort_by(|a, b| {
        let a_name = a
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let b_name = b
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        a_name.cmp(&b_name)
    });

    let current_index = files.iter().position(|file| {
        file.file_name()
            .map(|name| name.to_string_lossy() == current_name)
            .unwrap_or(false)
    });

    let Some(index) = current_index else {
        return Ok(None);
    };

    let next_index = if direction < 0 {
        index.checked_sub(1)
    } else if direction > 0 {
        let next = index + 1;
        (next < files.len()).then_some(next)
    } else {
        None
    };

    Ok(next_index.map(|index| files[index].to_string_lossy().to_string()))
}

#[tauri::command]
fn read_document(path: String) -> Result<DocumentPayload, String> {
    let path_buf = PathBuf::from(&path);
    let path_ref = path_buf.as_path();
    let file_name = path_ref
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    let extension = path_ref
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    let bytes = fs::read(path_ref).map_err(|error| format!("Unable to read file: {error}"))?;
    let source = String::from_utf8_lossy(&bytes).to_string();
    let base_info = build_file_info(path_ref, &file_name, &extension, &bytes);
    let source_character_count = source.chars().count();
    let source_line_count = source.lines().count();

    match extension.as_str() {
        "md" | "markdown" => Ok(DocumentPayload {
            path: Some(path),
            file_name: file_name.clone(),
            kind: DocumentKind::Markdown,
            title: file_name,
            source: source.clone(),
            markdown: Some(source),
            email: None,
            image: None,
            media: None,
            text: None,
            info: with_section(
                base_info,
                "Markdown",
                vec![
                    info_entry("Characters", source_character_count.to_string()),
                    info_entry("Lines", source_line_count.to_string()),
                ],
            ),
            error: None,
        }),
        "eml" => Ok(parse_email(path, file_name, &bytes, source, base_info)),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "ico" | "svg"
        | "avif" | "heic" | "heif" => Ok(parse_image(
            path,
            file_name,
            extension.as_str(),
            &bytes,
            source,
            base_info,
            &path_buf,
        )),
        "mp3" | "wav" | "ogg" | "oga" | "flac" | "m4a" | "aac" | "opus" => {
            Ok(parse_media(path, file_name, extension.as_str(), &bytes, DocumentKind::Audio, base_info))
        }
        "mp4" | "m4v" | "webm" | "ogv" | "mov" | "avi" | "mkv" => {
            Ok(parse_media(path, file_name, extension.as_str(), &bytes, DocumentKind::Video, base_info))
        }
        "docx" => Ok(parse_docx(path, file_name, &bytes, base_info)),
        ext if is_text_extension(ext) => Ok(parse_text(path, file_name, ext, source, base_info)),
        _ => Ok(DocumentPayload {
            path: Some(path),
            file_name,
            kind: DocumentKind::Unsupported,
            title: "Unsupported file".to_string(),
            source: String::new(),
            markdown: None,
            email: None,
            image: None,
            media: None,
            text: None,
            info: base_info,
            error: Some(
                "Rads Quick Viewer currently supports Markdown, .eml email, images, audio, video, text files, and .docx text extraction."
                    .to_string(),
            ),
        }),
    }
}

#[tauri::command]
fn open_default_app_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .arg("ms-settings:defaultapps")
            .spawn()
            .map_err(|error| format!("Unable to open Windows default-app settings: {error}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.general")
            .spawn()
            .map_err(|error| format!("Unable to open macOS system settings: {error}"))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let opened = Command::new("xdg-open")
            .arg("settings:///applications/default")
            .spawn()
            .is_ok();
        if !opened {
            return Err("Unable to open system default-app settings on this Linux desktop.".to_string());
        }
    }

    Ok(())
}

fn parse_email(
    path: String,
    file_name: String,
    bytes: &[u8],
    source: String,
    base_info: FileInfoPayload,
) -> DocumentPayload {
    match mailparse::parse_mail(bytes) {
        Ok(parsed) => {
            let headers = parsed
                .headers
                .iter()
                .map(|header| EmailHeader {
                    name: header.get_key().to_string(),
                    value: header.get_value(),
                })
                .collect::<Vec<_>>();

            let subject = parsed.headers.get_first_value("Subject");
            let email = EmailPayload {
                headers,
                from: format_address_header(&parsed, "From"),
                to: format_address_header(&parsed, "To"),
                date: parsed.headers.get_first_value("Date"),
                subject: subject.clone(),
                body_html: find_body_part(&parsed, "text/html"),
                body_text: find_body_part(&parsed, "text/plain").or_else(|| parsed.get_body().ok()),
            };
            let info_entries = email_info_entries(&subject, &email);
            let title = subject.clone().unwrap_or_else(|| "Email message".to_string());

            DocumentPayload {
                path: Some(path),
                file_name,
                kind: DocumentKind::Email,
                title,
                source,
                markdown: None,
                email: Some(email),
                image: None,
                media: None,
                text: None,
                info: with_section(base_info, "Email", info_entries),
                error: None,
            }
        }
        Err(error) => DocumentPayload {
            path: Some(path),
            file_name,
            kind: DocumentKind::Email,
            title: "Email message".to_string(),
            source,
            markdown: None,
            email: None,
            image: None,
            media: None,
            text: None,
            info: base_info,
            error: Some(format!("Unable to parse email: {error}")),
        },
    }
}

fn parse_image(
    path: String,
    file_name: String,
    extension: &str,
    bytes: &[u8],
    source: String,
    base_info: FileInfoPayload,
    path_ref: &Path,
) -> DocumentPayload {
    let (data_url, mime_type, render_note) = match image_data_url(extension, bytes) {
        Ok(payload) => payload,
        Err(error) => (
            String::new(),
            mime_for_image_extension(extension).to_string(),
            Some(error),
        ),
    };

    DocumentPayload {
        path: Some(path),
        file_name: file_name.clone(),
        kind: DocumentKind::Image,
        title: file_name,
        source,
        markdown: None,
        email: None,
        image: Some(ImagePayload {
            data_url,
            mime_type: mime_type.clone(),
            render_note,
        }),
        media: None,
        text: None,
        info: with_image_info(base_info, path_ref, extension, bytes, &mime_type),
        error: None,
    }
}

fn parse_media(
    path: String,
    file_name: String,
    extension: &str,
    bytes: &[u8],
    kind: DocumentKind,
    base_info: FileInfoPayload,
) -> DocumentPayload {
    let mime_type = mime_for_media_extension(extension).to_string();
    let render_note = media_render_note(extension).map(str::to_string);
    DocumentPayload {
        path: Some(path),
        file_name: file_name.clone(),
        kind,
        title: file_name,
        source: String::new(),
        markdown: None,
        email: None,
        image: None,
        media: Some(MediaPayload {
            data_url: format!("data:{};base64,{}", mime_type, general_purpose::STANDARD.encode(bytes)),
            mime_type: mime_type.clone(),
            render_note,
        }),
        text: None,
        info: with_section(
            base_info,
            "Playback",
            vec![
                info_entry("MIME type", mime_type),
                info_entry(
                    "Codec support",
                    media_render_note(extension).unwrap_or("Handled by WebView2 media playback").to_string(),
                ),
            ],
        ),
        error: None,
    }
}

fn image_data_url(extension: &str, bytes: &[u8]) -> Result<(String, String, Option<String>), String> {
    match extension {
        "bmp" | "tif" | "tiff" | "ico" => {
            let image = image::load_from_memory(bytes)
                .map_err(|error| format!("Unable to decode image for preview: {error}"))?;
            let mut encoded = Cursor::new(Vec::new());
            image
                .write_to(&mut encoded, ImageFormat::Png)
                .map_err(|error| format!("Unable to convert image for preview: {error}"))?;
            Ok((
                format!(
                    "data:image/png;base64,{}",
                    general_purpose::STANDARD.encode(encoded.into_inner())
                ),
                "image/png".to_string(),
                Some("Converted to PNG for preview.".to_string()),
            ))
        }
        "heic" | "heif" => Ok((
            format!(
                "data:{};base64,{}",
                mime_for_image_extension(extension),
                general_purpose::STANDARD.encode(bytes)
            ),
            mime_for_image_extension(extension).to_string(),
            Some("HEIC/HEIF preview depends on WebView2 and installed Windows codec support.".to_string()),
        )),
        _ => Ok((
            format!(
                "data:{};base64,{}",
                mime_for_image_extension(extension),
                general_purpose::STANDARD.encode(bytes)
            ),
            mime_for_image_extension(extension).to_string(),
            None,
        )),
    }
}

fn parse_text(
    path: String,
    file_name: String,
    extension: &str,
    source: String,
    base_info: FileInfoPayload,
) -> DocumentPayload {
    let language = language_for_extension(extension).map(str::to_string);
    let character_count = source.chars().count();
    let line_count = source.lines().count();
    DocumentPayload {
        path: Some(path),
        file_name: file_name.clone(),
        kind: DocumentKind::Text,
        title: file_name,
        source: source.clone(),
        markdown: None,
        email: None,
        image: None,
        media: None,
        text: Some(TextPayload {
            text: source,
            language: language.clone(),
        }),
        info: with_section(
            base_info,
            "Text",
            vec![
                optional_info_entry("Language", language.as_deref()),
                Some(info_entry("Characters", character_count.to_string())),
                Some(info_entry("Lines", line_count.to_string())),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
        error: None,
    }
}

fn parse_docx(path: String, file_name: String, bytes: &[u8], base_info: FileInfoPayload) -> DocumentPayload {
    let extracted = extract_docx_text(bytes);
    let (text, error) = match extracted {
        Ok(text) => (text, None),
        Err(error) => (String::new(), Some(error)),
    };
    let character_count = text.chars().count();
    let line_count = text.lines().count();

    DocumentPayload {
        path: Some(path),
        file_name: file_name.clone(),
        kind: DocumentKind::Docx,
        title: file_name,
        source: String::new(),
        markdown: None,
        email: None,
        image: None,
        media: None,
        text: Some(TextPayload {
            text,
            language: None,
        }),
        info: with_section(
            base_info,
            "Document Text",
            vec![
                info_entry("Characters", character_count.to_string()),
                info_entry("Lines", line_count.to_string()),
            ],
        ),
        error,
    }
}

fn extract_docx_text(bytes: &[u8]) -> Result<String, String> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| format!("Unable to read .docx archive: {error}"))?;
    let mut document = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| format!("Unable to find word/document.xml: {error}"))?
        .read_to_string(&mut document)
        .map_err(|error| format!("Unable to read document text: {error}"))?;

    Ok(extract_text_from_word_xml(&document))
}

fn extract_text_from_word_xml(xml: &str) -> String {
    let mut output = String::new();
    let mut in_text = false;
    let mut buffer = String::new();
    let mut chars = xml.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            for tag_ch in chars.by_ref() {
                if tag_ch == '>' {
                    break;
                }
                tag.push(tag_ch);
            }

            let tag_name = tag
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_start_matches('/');
            if tag_name == "w:t" {
                in_text = !tag.starts_with('/');
                if !in_text {
                    output.push_str(&decode_xml_entities(&buffer));
                    buffer.clear();
                }
            } else if tag_name == "w:tab" {
                output.push('\t');
            } else if tag_name == "w:br" || tag_name == "w:p" {
                if !output.ends_with('\n') && !output.is_empty() {
                    output.push('\n');
                }
            }
        } else if in_text {
            buffer.push(ch);
        }
    }

    if !buffer.is_empty() {
        output.push_str(&decode_xml_entities(&buffer));
    }

    output.trim().to_string()
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn build_file_info(path: &Path, file_name: &str, extension: &str, bytes: &[u8]) -> FileInfoPayload {
    let metadata = fs::metadata(path).ok();
    let modified = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    FileInfoPayload {
        general: vec![
            info_entry("Name", file_name.to_string()),
            info_entry("Type", extension.to_uppercase()),
            info_entry("Size", format_file_size(bytes.len() as u64)),
            info_entry("Modified", modified),
            info_entry("Path", path.to_string_lossy().to_string()),
        ],
        sections: Vec::new(),
    }
}

fn with_section(mut info: FileInfoPayload, title: &str, entries: Vec<InfoEntry>) -> FileInfoPayload {
    if !entries.is_empty() {
        info.sections.push(InfoSection {
            title: title.to_string(),
            entries,
        });
    }

    info
}

fn info_entry(label: &str, value: String) -> InfoEntry {
    InfoEntry {
        label: label.to_string(),
        value,
    }
}

fn optional_info_entry(label: &str, value: Option<&str>) -> Option<InfoEntry> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| info_entry(label, value.to_string()))
}

fn email_info_entries(subject: &Option<String>, email: &EmailPayload) -> Vec<InfoEntry> {
    vec![
        optional_info_entry("Subject", subject.as_deref()),
        optional_info_entry("From", email.from.as_deref()),
        optional_info_entry("To", email.to.as_deref()),
        optional_info_entry("Date", email.date.as_deref()),
        Some(info_entry("Headers", email.headers.len().to_string())),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn with_image_info(
    mut info: FileInfoPayload,
    path: &Path,
    extension: &str,
    bytes: &[u8],
    mime_type: &str,
) -> FileInfoPayload {
    let mut image_entries = vec![info_entry("MIME type", mime_type.to_string())];

    if let Ok((width, height)) = image::image_dimensions(path) {
        image_entries.push(info_entry("Resolution", format!("{width} x {height} px")));
        image_entries.push(info_entry("Megapixels", format!("{:.2}", (width as f64 * height as f64) / 1_000_000.0)));
    }

    if let Some((x_dpi, y_dpi)) = image_dpi(extension, bytes) {
        image_entries.push(info_entry("DPI", format!("{x_dpi:.0} x {y_dpi:.0}")));
    }

    info = with_section(info, "Image", image_entries);

    let exif_entries = image_exif_entries(bytes);
    with_section(info, "EXIF", exif_entries)
}

fn image_dpi(extension: &str, bytes: &[u8]) -> Option<(f64, f64)> {
    match extension {
        "png" => png_dpi(bytes),
        "jpg" | "jpeg" => jpeg_dpi(bytes),
        _ => None,
    }
}

fn png_dpi(bytes: &[u8]) -> Option<(f64, f64)> {
    const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 33 || &bytes[0..8] != SIGNATURE {
        return None;
    }

    let mut index = 8;
    while index + 12 <= bytes.len() {
        let length = u32::from_be_bytes(bytes[index..index + 4].try_into().ok()?) as usize;
        let chunk_type = &bytes[index + 4..index + 8];
        let data_start = index + 8;
        let data_end = data_start + length;
        if data_end + 4 > bytes.len() {
            return None;
        }

        if chunk_type == b"pHYs" && length == 9 {
            let x_ppm = u32::from_be_bytes(bytes[data_start..data_start + 4].try_into().ok()?) as f64;
            let y_ppm = u32::from_be_bytes(bytes[data_start + 4..data_start + 8].try_into().ok()?) as f64;
            let unit = bytes[data_start + 8];
            if unit == 1 {
                return Some((x_ppm * 0.0254, y_ppm * 0.0254));
            }
        }

        index = data_end + 4;
    }

    None
}

fn jpeg_dpi(bytes: &[u8]) -> Option<(f64, f64)> {
    if bytes.len() < 20 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }

    let mut index = 2;
    while index + 4 < bytes.len() {
        if bytes[index] != 0xff {
            return None;
        }
        let marker = bytes[index + 1];
        index += 2;

        if marker == 0xda || marker == 0xd9 {
            break;
        }

        let length = u16::from_be_bytes(bytes[index..index + 2].try_into().ok()?) as usize;
        if length < 2 || index + length > bytes.len() {
            return None;
        }

        let segment_start = index + 2;
        if marker == 0xe0 && length >= 16 && &bytes[segment_start..segment_start + 5] == b"JFIF\0" {
            let units = bytes[segment_start + 7];
            let x_density = u16::from_be_bytes(bytes[segment_start + 8..segment_start + 10].try_into().ok()?) as f64;
            let y_density = u16::from_be_bytes(bytes[segment_start + 10..segment_start + 12].try_into().ok()?) as f64;
            return match units {
                1 => Some((x_density, y_density)),
                2 => Some((x_density * 2.54, y_density * 2.54)),
                _ => None,
            };
        }

        index += length;
    }

    None
}

fn image_exif_entries(bytes: &[u8]) -> Vec<InfoEntry> {
    let mut cursor = Cursor::new(bytes);
    let Ok(exif) = Reader::new().read_from_container(&mut cursor) else {
        return Vec::new();
    };

    let tags = [
        ("Camera Make", Tag::Make),
        ("Camera Model", Tag::Model),
        ("Lens", Tag::LensModel),
        ("Date Taken", Tag::DateTimeOriginal),
        ("Exposure", Tag::ExposureTime),
        ("F-number", Tag::FNumber),
        ("ISO", Tag::PhotographicSensitivity),
        ("Focal Length", Tag::FocalLength),
        ("Orientation", Tag::Orientation),
        ("GPS Latitude", Tag::GPSLatitude),
        ("GPS Longitude", Tag::GPSLongitude),
    ];

    tags.iter()
        .filter_map(|(label, tag)| {
            exif.get_field(*tag, In::PRIMARY)
                .map(|field| info_entry(label, field.display_value().with_unit(&exif).to_string()))
        })
        .collect()
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{size:.2} {}", UNITS[unit])
    }
}

fn find_body_part(mail: &ParsedMail<'_>, mime_type: &str) -> Option<String> {
    if mail.ctype.mimetype.eq_ignore_ascii_case(mime_type) {
        return mail.get_body().ok();
    }

    mail.subparts
        .iter()
        .find_map(|part| find_body_part(part, mime_type))
}

fn format_address_header(mail: &ParsedMail<'_>, name: &str) -> Option<String> {
    let header = mail.headers.get_first_header(name)?;
    match addrparse_header(&header) {
        Ok(addresses) => Some(addresses.to_string()),
        Err(_) => Some(header.get_value()),
    }
}

fn supported_extensions() -> Vec<&'static str> {
    vec![
        "md", "markdown", "eml", "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff",
        "ico", "svg", "avif", "heic", "heif", "txt", "text", "log", "csv", "tsv", "json",
        "xml", "yaml", "yml", "toml", "ini", "html", "htm", "css", "js", "jsx", "ts", "tsx",
        "rs", "py", "php", "rb", "go", "java", "c", "cpp", "h", "hpp", "cs", "sql", "docx",
        "mp3", "wav", "ogg", "oga", "flac", "m4a", "aac", "opus", "mp4", "m4v", "webm", "ogv",
        "mov", "avi", "mkv",
    ]
}

fn mime_for_media_extension(extension: &str) -> &'static str {
    match extension {
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "opus" => "audio/opus",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        _ => "application/octet-stream",
    }
}

fn media_render_note(extension: &str) -> Option<&'static str> {
    match extension {
        "avi" | "mkv" | "mov" | "flac" | "aac" | "opus" => {
            Some("Playback depends on WebView2 and installed OS codec support.")
        }
        _ => None,
    }
}

fn is_text_extension(extension: &str) -> bool {
    matches!(
        extension,
        "txt"
            | "text"
            | "log"
            | "csv"
            | "tsv"
            | "json"
            | "xml"
            | "yaml"
            | "yml"
            | "toml"
            | "ini"
            | "html"
            | "htm"
            | "css"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "rs"
            | "py"
            | "php"
            | "rb"
            | "go"
            | "java"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "cs"
            | "sql"
    )
}

fn mime_for_image_extension(extension: &str) -> &'static str {
    match extension {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "heic" => "image/heic",
        "heif" => "image/heif",
        _ => "application/octet-stream",
    }
}

fn language_for_extension(extension: &str) -> Option<&'static str> {
    match extension {
        "json" => Some("JSON"),
        "xml" => Some("XML"),
        "yaml" | "yml" => Some("YAML"),
        "toml" => Some("TOML"),
        "html" | "htm" => Some("HTML"),
        "css" => Some("CSS"),
        "js" | "jsx" => Some("JavaScript"),
        "ts" | "tsx" => Some("TypeScript"),
        "rs" => Some("Rust"),
        "py" => Some("Python"),
        "php" => Some("PHP"),
        "rb" => Some("Ruby"),
        "go" => Some("Go"),
        "java" => Some("Java"),
        "c" | "h" => Some("C"),
        "cpp" | "hpp" => Some("C++"),
        "cs" => Some("C#"),
        "sql" => Some("SQL"),
        "csv" => Some("CSV"),
        "tsv" => Some("TSV"),
        _ => None,
    }
}

pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            choose_file,
            get_sibling_file,
            get_initial_file,
            open_default_app_settings,
            read_document
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rads Viewer");
}
