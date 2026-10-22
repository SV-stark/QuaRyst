use iced::Element;
use iced::widget::{Text, text};

pub const FONT: iced::Font = iced::Font::with_name("lucide");

// Load the pre-built Lucide font from assets
pub const FONT_BYTES: &[u8] = include_bytes!("../../assets/lucide.ttf");

// Load Inter variable font for polished typography
pub const INTER_FONT_BYTES: &[u8] = include_bytes!("../../assets/InterVariable.ttf");
pub const INTER_FONT: iced::Font = iced::Font::with_name("Inter");

/// Icon name to Unicode character mapping for Lucide font
pub fn get_icon_char(name: &str) -> char {
    match name {
        "magnifying-glass" | "search" => '\u{e151}',
        "magnifying-glass-plus" => '\u{e1b6}',
        "magnifying-glass-minus" => '\u{e1b7}',
        "gear" | "settings" | "sliders" => '\u{e154}',
        "trash" => '\u{e18d}',
        "file" => '\u{e0c0}',
        "file-text" | "text" => '\u{e0cc}',
        "file-code" | "code" => '\u{e0c3}',
        "file-image" | "image" => '\u{e0c6}',
        "file-audio" | "audio" => '\u{e0c7}',
        "file-video" | "video" => '\u{e0c8}',
        "folder" => '\u{e0d7}',
        "folder-open" => '\u{e247}',
        "info" => '\u{e0f9}',
        "warning" => '\u{e193}',
        "check" => '\u{e06c}',
        "x" | "close" => '\u{e1b2}',
        "plus" => '\u{e13d}',
        "minus" => '\u{e11c}',
        "arrow-right" => '\u{e049}',
        "arrow-left" => '\u{e048}',
        "arrow-up" => '\u{e04a}',
        "arrow-down" => '\u{e042}',
        "chevron-left" => '\u{e073}',
        "chevron-right" => '\u{e074}',
        "chevron-down" => '\u{e070}',
        "chevron-up" => '\u{e077}',
        "download" => '\u{e0b2}',
        "copy" => '\u{e09e}',
        "database" => '\u{e0ad}',
        "star" => '\u{e176}',
        "sun" => '\u{e179}',
        "moon" => '\u{e11e}',
        "filter" => '\u{e0cb}',
        "clock" => '\u{e087}',
        "external-link" | "external" => '\u{e0b7}',
        "sparkles" => '\u{e172}',
        "eye" => '\u{e0bd}',
        "tag" => '\u{e181}',
        "calendar" => '\u{e063}',
        "keyboard" => '\u{e100}',
        "refresh" => '\u{e146}',
        _ => {
            static WARNED_ICONS: std::sync::Mutex<Option<std::collections::HashSet<String>>> =
                std::sync::Mutex::new(None);
            if let Ok(mut lock) = WARNED_ICONS.lock() {
                let set = lock.get_or_insert_with(std::collections::HashSet::new);
                if set.insert(name.to_string()) {
                    tracing::warn!("icon '{}' not found", name);
                }
            }
            '\u{003f}' // Question mark as fallback
        }
    }
}

/// Helper function to create an iced Text widget containing the given icon unicode.
#[must_use]
pub fn icon<'a>(name: &str) -> Text<'a> {
    text(get_icon_char(name).to_string()).font(FONT)
}

/// Maps a file extension to a Lucide icon name for visual file-type identification.
#[must_use]
pub fn extension_icon(ext: Option<&str>) -> &'static str {
    match ext.map(str::to_ascii_lowercase).as_deref() {
        Some(
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "c" | "cpp" | "h" | "hpp" | "java"
            | "cs" | "html" | "css" | "scss" | "less",
        ) => "file-code",
        Some("json" | "yaml" | "yml" | "toml" | "xml") => "file-json",
        Some(
            "md" | "markdown" | "rst" | "txt" | "pdf" | "doc" | "docx" | "odt" | "rtf" | "log",
        ) => "file-text",
        Some("xls" | "xlsx" | "ods" | "csv") => "file-spreadsheet",
        Some("ppt" | "pptx" | "odp") => "presentation",
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" | "tiff") => {
            "file-image"
        }
        Some("mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a") => "music",
        Some("mp4" | "avi" | "mkv" | "mov" | "webm") => "video",
        Some("zip" | "tar" | "gz" | "7z" | "rar" | "bz2") => "archive",
        Some("exe" | "msi" | "dll" | "so" | "dylib") => "cog",
        Some("sh" | "bash" | "zsh" | "fish" | "bat" | "cmd" | "ps1") => "terminal",
        _ => "file",
    }
}

/// For the old `load_icon` helper signature
#[must_use]
pub fn load_icon<'a>(name: &str) -> Element<'a, super::Message> {
    icon(name).size(16).into()
}

#[must_use]
pub fn load_icon_size<'a>(name: &str, size: f32) -> Element<'a, super::Message> {
    icon(name).size(size).into()
}
