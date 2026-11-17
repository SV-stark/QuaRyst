use crate::error::{FlashError, Result};
use std::path::{Path, PathBuf};

pub mod memory_map;

use compact_str::CompactString;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParsedDocument {
    pub path: String,
    pub content: String,
    pub title: Option<CompactString>,
    pub language: Option<CompactString>,
    pub keywords: Option<String>,
    pub layout: Option<String>,
    pub code_metadata: Option<String>,
    pub embeddings: Option<Vec<f32>>,
}

#[derive(Debug, Clone)]
pub struct PreviewElement {
    pub element_type: crate::models::ElementType,
    pub content: String,
}

/// Returns true for plaintext, code, data, and configuration file formats
/// that can be read with zero-copy UTF-8 validation bypassing heavy document parsing.
#[must_use]
pub fn is_plaintext_fast_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "txt"
            | "md"
            | "markdown"
            | "rs"
            | "py"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "xml"
            | "csv"
            | "tsv"
            | "html"
            | "htm"
            | "css"
            | "scss"
            | "sass"
            | "less"
            | "c"
            | "cpp"
            | "cc"
            | "cxx"
            | "h"
            | "hpp"
            | "hh"
            | "hxx"
            | "cs"
            | "go"
            | "java"
            | "kt"
            | "kts"
            | "swift"
            | "php"
            | "rb"
            | "lua"
            | "sh"
            | "bash"
            | "zsh"
            | "bat"
            | "cmd"
            | "ps1"
            | "psm1"
            | "ini"
            | "conf"
            | "config"
            | "env"
            | "log"
            | "sql"
            | "graphql"
            | "proto"
            | "diff"
            | "patch"
            | "tex"
            | "r"
            | "dart"
            | "scala"
            | "zig"
            | "nim"
    )
}

/// True for extensions that should be previewed with syntax highlighting.
#[must_use]
pub fn is_code_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "rs" | "py"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "cs"
            | "go"
            | "java"
            | "kt"
            | "swift"
            | "php"
            | "rb"
            | "lua"
            | "sh"
            | "sql"
            | "json"
            | "toml"
            | "yaml"
            | "yml"
            | "xml"
            | "html"
            | "css"
    )
}

/// Pre-warms any parser registries (no-op for pure-Rust native extractors).
pub const fn ensure_initialized() {}

/// Streams a file through BLAKE3 without loading it into memory.
///
/// The heavy extraction path used to `read_file` the *entire* document a second
/// time purely to hash it, which for a 90 MB PDF meant another 90 MB of pages
/// faulted in and a transient allocation of the same size. Hashing in 256 KiB
/// chunks keeps this path's memory flat and lets the OS evict the previous
/// buffer before the next one is read.
fn hash_file_streaming(path: &Path) -> Result<[u8; 32]> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| FlashError::Io(std::sync::Arc::new(e)))?;

    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 256 * 1024];
    let mut reader = std::io::BufReader::with_capacity(256 * 1024, file);

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| FlashError::Io(std::sync::Arc::new(e)))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(*hasher.finalize().as_bytes())
}

fn decode_utf16(bytes: &[u8], big_endian: bool) -> String {
    let units = bytes.as_chunks::<2>().0.iter().map(|&c| {
        if big_endian {
            u16::from_be_bytes(c)
        } else {
            u16::from_le_bytes(c)
        }
    });
    char::decode_utf16(units)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}

/// Decodes raw text bytes, honouring UTF-8/UTF-16 BOMs, rejecting binary
/// payloads (null byte in the first 8 KiB), and falling back to Latin-1.
#[must_use]
pub fn decode_text_bytes(bytes: &[u8]) -> Option<String> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return Some(String::from_utf8_lossy(rest).into_owned());
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return Some(decode_utf16(rest, false));
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return Some(decode_utf16(rest, true));
    }
    let head = &bytes[..bytes.len().min(8192)];
    if head.contains(&0) {
        return None;
    }
    Some(std::str::from_utf8(bytes).map_or_else(
        |_| bytes.iter().map(|&b| b as char).collect(),
        ToString::to_string,
    ))
}

/// Reads a plaintext/code file and returns its decoded content plus BLAKE3 hash in
/// a single pass over the data.
fn parse_plaintext(path: &Path) -> Result<(String, [u8; 32])> {
    let file_data = memory_map::read_file(path)?;
    let hash = *blake3::hash(&file_data).as_bytes();
    let content = decode_text_bytes(&file_data)
        .ok_or_else(|| FlashError::parse(path, "Binary content detected".to_string()))?;
    Ok((content, hash))
}

/// Builds a `ParsedDocument` for a plaintext file, using the file name as title.
fn plaintext_document(path: &Path, content: String) -> ParsedDocument {
    ParsedDocument {
        path: path.to_string_lossy().to_string(),
        content,
        title: path
            .file_name()
            .map(|n| CompactString::from(n.to_string_lossy().as_ref())),
        language: None,
        keywords: None,
        layout: None,
        code_metadata: None,
        embeddings: None,
    }
}

/// Synchronously extracts a document using pure-Rust native parsers
/// (`unpdf` for PDFs, `zip` + XML tag stripping for Office/ODF/EPUB,
/// `strip_rtf`, `strip_tags`, and `legacy_binary` for `.doc`/`.xls`/`.ppt`).
pub fn parse_file_sync(path: &Path) -> Result<ParsedDocument> {
    if is_plaintext_fast_path(path) {
        let (content, _) = parse_plaintext(path)?;
        return Ok(plaintext_document(path, content));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    let (content, title) = match ext.as_str() {
        "pdf" => extract_pdf(path)?,
        "docx" | "docm" | "dotx" | "dotm" => (
            zip_xml(path, |n| {
                n == "word/document.xml"
                    || is_xml_part(n, "word/header")
                    || is_xml_part(n, "word/footer")
                    || n == "word/footnotes.xml"
            })?,
            None,
        ),
        "xlsx" | "xlsm" | "xltx" | "xltm" => (
            zip_xml(path, |n| {
                n == "xl/sharedStrings.xml" || is_xml_part(n, "xl/worksheets/sheet")
            })?,
            None,
        ),
        "pptx" | "pptm" | "ppsx" | "potx" | "potm" => (
            zip_xml(path, |n| {
                is_xml_part(n, "ppt/slides/slide") || is_xml_part(n, "ppt/notesSlides/notesSlide")
            })?,
            None,
        ),
        "odt" | "ods" | "odp" => (zip_xml(path, |n| n == "content.xml")?, None),
        "epub" => (
            zip_xml(path, |n| {
                std::path::Path::new(n).extension().is_some_and(|e| {
                    e.eq_ignore_ascii_case("xhtml")
                        || e.eq_ignore_ascii_case("html")
                        || e.eq_ignore_ascii_case("htm")
                })
            })?,
            None,
        ),
        "doc" | "xls" | "ppt" => (legacy_binary(path)?, None),
        "xhtml" | "xht" | "svg" | "kml" => {
            let bytes = memory_map::read_file(path)?;
            let raw = decode_text_bytes(&bytes)
                .ok_or_else(|| FlashError::parse(path, "Binary content detected".to_string()))?;
            (strip_tags(&raw), None)
        }
        "rtf" => {
            let bytes = memory_map::read_file(path)?;
            let raw = decode_text_bytes(&bytes)
                .ok_or_else(|| FlashError::parse(path, "Binary content detected".to_string()))?;
            (strip_rtf(&raw), None)
        }
        _ => {
            return Err(FlashError::parse(
                path,
                format!("Unsupported file format: .{ext}"),
            ));
        }
    };

    let fallback_title = path
        .file_name()
        .map(|n| CompactString::from(n.to_string_lossy().as_ref()));

    Ok(ParsedDocument {
        path: path.to_string_lossy().to_string(),
        content,
        title: title.or(fallback_title),
        language: None,
        keywords: None,
        layout: None,
        code_metadata: None,
        embeddings: None,
    })
}

/// Detect file type, route to appropriate native parser, and compute BLAKE3 hash.
pub async fn parse_file_with_hash(
    path: &Path,
    _enable_ocr: bool,
) -> Result<(ParsedDocument, [u8; 32])> {
    let path_owned = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        if is_plaintext_fast_path(&path_owned) {
            let (content, hash) = parse_plaintext(&path_owned)?;
            return Ok((plaintext_document(&path_owned, content), hash));
        }
        let doc = parse_file_sync(&path_owned)?;
        let hash = hash_file_streaming(&path_owned)?;
        Ok((doc, hash))
    })
    .await
    .map_err(|e| FlashError::parse(path, format!("Parse task panicked: {e}")))?
}

/// Detect file type and route to appropriate parser.
pub async fn parse_file(path: &Path, enable_ocr: bool) -> Result<ParsedDocument> {
    parse_file_with_hash(path, enable_ocr)
        .await
        .map(|(doc, _)| doc)
}

/// Extracts structured preview elements for the UI preview pane.
pub async fn parse_file_preview(path: &Path, _enable_ocr: bool) -> Result<Vec<PreviewElement>> {
    if is_plaintext_fast_path(path) {
        let (content, _) = parse_plaintext(path)?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let element_type = if is_code_extension(extension) {
            crate::models::ElementType::CodeBlock
        } else {
            crate::models::ElementType::NarrativeText
        };

        return Ok(vec![PreviewElement {
            element_type,
            content,
        }]);
    }

    let path_owned = path.to_path_buf();
    let doc = tokio::task::spawn_blocking(move || parse_file_sync(&path_owned))
        .await
        .map_err(|e| FlashError::parse(path, format!("Preview task failed: {e}")))??;

    let elements = doc
        .content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| PreviewElement {
            element_type: crate::models::ElementType::NarrativeText,
            content: line.to_string(),
        })
        .collect();

    Ok(elements)
}

/// One file's parse outcome: either the document plus its content hash, or the
/// reason it could not be parsed.
pub type ParseOutcome = Result<(ParsedDocument, [u8; 32])>;

/// Outcome for a whole batch, positionally matching the input `paths`.
pub type BatchParseResult = Vec<ParseOutcome>;

/// Parses a batch of files in parallel on the Rayon pool inside `spawn_blocking`.
pub async fn parse_files_batch(
    paths: &[PathBuf],
    _max_threads: u8,
    _enable_ocr: bool,
) -> Result<BatchParseResult> {
    use rayon::prelude::*;

    if paths.is_empty() {
        return Ok(Vec::new());
    }

    let owned: Vec<PathBuf> = paths.to_vec();
    tokio::task::spawn_blocking(move || {
        owned
            .par_iter()
            .map(|path| {
                if is_plaintext_fast_path(path) {
                    let (content, hash) = parse_plaintext(path)?;
                    Ok((plaintext_document(path, content), hash))
                } else {
                    let doc = parse_file_sync(path)?;
                    let hash = hash_file_streaming(path)?;
                    Ok((doc, hash))
                }
            })
            .collect()
    })
    .await
    .map_err(|e| {
        FlashError::Io(std::sync::Arc::new(std::io::Error::other(format!(
            "Batch parse pool panicked: {e}"
        ))))
    })
}

fn extract_pdf(path: &Path) -> Result<(String, Option<CompactString>)> {
    let options = unpdf::ParseOptions::new().lenient().with_resources(false);
    let doc = unpdf::parse_file_with_options(path, options)
        .map_err(|e| FlashError::parse(path, format!("PDF parse failed: {e}")))?;
    let title = doc
        .metadata
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(CompactString::from);
    let text = unpdf::render::to_text(&doc, &unpdf::RenderOptions::default())
        .map_err(|e| FlashError::parse(path, format!("PDF render failed: {e}")))?;
    Ok((text, title))
}

fn is_xml_part(name: &str, prefix: &str) -> bool {
    name.starts_with(prefix)
        && std::path::Path::new(name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("xml"))
}

fn natural_key(s: &str) -> (String, u64) {
    let digits: String = s
        .chars()
        .rev()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let stem: String = s.chars().filter(|c| !c.is_ascii_digit()).collect();
    (stem, digits.parse().unwrap_or(0))
}

fn zip_xml(path: &Path, want: impl Fn(&str) -> bool) -> Result<String> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| FlashError::Io(std::sync::Arc::new(e)))?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| FlashError::parse(path, format!("ZIP open failed: {e}")))?;

    let mut names: Vec<String> = archive
        .file_names()
        .filter_map(std::result::Result::ok)
        .filter(|n| want(n))
        .map(std::borrow::Cow::into_owned)
        .collect();
    names.sort_by_key(|a| natural_key(a));

    let mut out = String::new();
    let mut buf = String::new();
    for name in names {
        let Ok(entry) = archive.by_name(&name) else {
            continue;
        };
        if entry.size() > (128 << 20) {
            continue;
        }
        buf.clear();
        if entry.take(128 << 20).read_to_string(&mut buf).is_ok() {
            out.push_str(&strip_tags(&buf));
            out.push('\n');
        }
    }
    Ok(out)
}

/// Removes XML/HTML tags, turning block-ending tags into line breaks and decoding entities.
#[allow(clippy::too_many_lines)]
fn strip_tags(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len() / 3);
    let mut chars = xml.char_indices().peekable();
    let bytes = xml.as_bytes();
    let mut skip_until: Option<&'static str> = None;

    while let Some((i, c)) = chars.next() {
        if let Some(end) = skip_until {
            if c == '<'
                && xml[i..].len() >= end.len()
                && xml[i..i + end.len()].eq_ignore_ascii_case(end)
            {
                skip_until = None;
            } else {
                continue;
            }
        }

        if c == '<' {
            let rest = &xml[i + 1..];
            let close = rest.starts_with('/');
            let name_start = usize::from(close);
            let name: String = rest[name_start..]
                .chars()
                .take_while(|ch| !ch.is_whitespace() && *ch != '>' && *ch != '/')
                .collect();
            let lname = name.to_ascii_lowercase();
            if !close && (lname == "script" || lname == "style") {
                skip_until = Some(if lname == "script" {
                    "</script"
                } else {
                    "</style"
                });
            }
            let mut end_idx = i + 1;
            while end_idx < bytes.len() && bytes[end_idx] != b'>' {
                end_idx += 1;
            }
            while let Some(&(k, _)) = chars.peek() {
                if k <= end_idx {
                    chars.next();
                } else {
                    break;
                }
            }
            let local = lname.rsplit(':').next().unwrap_or("");
            let newline = (close
                && matches!(
                    local,
                    "p" | "div"
                        | "tr"
                        | "li"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "row"
                        | "si"
                        | "title"
                        | "table-row"
                        | "h"
                        | "section"
                        | "article"
                ))
                || matches!(local, "br" | "cr" | "line-break");
            let space = (close && matches!(local, "c" | "td" | "th" | "table-cell" | "v"))
                || (!close && matches!(local, "tab" | "s"));
            if newline {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            } else if space && !out.ends_with(' ') && !out.ends_with('\n') {
                out.push(' ');
            }
            continue;
        }

        if c == '&' {
            let rest = &xml[i..];
            if let Some(semi) = rest[..rest.len().min(12)].find(';') {
                let ent = &rest[1..semi];
                let decoded = match ent {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    "nbsp" => Some(' '),
                    _ if ent.starts_with("#x") || ent.starts_with("#X") => {
                        u32::from_str_radix(&ent[2..], 16)
                            .ok()
                            .and_then(char::from_u32)
                    }
                    _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
                    _ => None,
                };
                if let Some(d) = decoded {
                    out.push(d);
                    for _ in 0..semi {
                        chars.next();
                    }
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

/// Minimal RTF text extractor: drops control words and metadata groups, keeps text.
fn strip_rtf(rtf: &str) -> String {
    if !rtf.starts_with("{\\rtf") {
        return rtf.to_string();
    }
    let mut out = String::new();
    let chars: Vec<char> = rtf.chars().collect();
    let mut i = 0;
    let mut skip_depth: Vec<bool> = vec![false];
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '{' => {
                let parent = *skip_depth.last().unwrap_or(&false);
                skip_depth.push(parent);
                i += 1;
            }
            '}' => {
                skip_depth.pop();
                i += 1;
            }
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    break;
                }
                let next = chars[i];
                if next.is_ascii_alphabetic() {
                    let start = i;
                    while i < chars.len() && chars[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word: String = chars[start..i].iter().collect();
                    let mut num = String::new();
                    if i < chars.len() && (chars[i] == '-' || chars[i].is_ascii_digit()) {
                        while i < chars.len() && (chars[i] == '-' || chars[i].is_ascii_digit()) {
                            num.push(chars[i]);
                            i += 1;
                        }
                    }
                    if i < chars.len() && chars[i] == ' ' {
                        i += 1;
                    }
                    let skipping = *skip_depth.last().unwrap_or(&false);
                    match word.as_str() {
                        "par" | "line" if !skipping => out.push('\n'),
                        "tab" if !skipping => out.push(' '),
                        "fonttbl" | "colortbl" | "stylesheet" | "info" | "pict" | "header"
                        | "footer" => {
                            if let Some(last) = skip_depth.last_mut() {
                                *last = true;
                            }
                        }
                        "u" if !skipping => {
                            if let Ok(v) = num.parse::<i32>() {
                                let code = u32::try_from(if v < 0 { v + 65536 } else { v }).ok();
                                if let Some(decoded) = code.and_then(char::from_u32) {
                                    out.push(decoded);
                                }
                                if i < chars.len() && chars[i] == '?' {
                                    i += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                } else if next == '*' {
                    if let Some(last) = skip_depth.last_mut() {
                        *last = true;
                    }
                    i += 1;
                } else if next == '\'' && i + 2 < chars.len() {
                    let hex: String = chars[i + 1..i + 3].iter().collect();
                    if let Ok(v) = u8::from_str_radix(&hex, 16)
                        && !*skip_depth.last().unwrap_or(&false)
                    {
                        out.push(v as char);
                    }
                    i += 3;
                } else {
                    if !*skip_depth.last().unwrap_or(&false) && matches!(next, '\\' | '{' | '}') {
                        out.push(next);
                    }
                    i += 1;
                }
            }
            '\r' | '\n' => i += 1,
            _ => {
                if !*skip_depth.last().unwrap_or(&false) {
                    out.push(ch);
                }
                i += 1;
            }
        }
    }
    out
}

/// Best-effort text extraction from legacy binary Office files (`.doc`/`.xls`/`.ppt`):
/// collects runs of readable UTF-16LE and ASCII text.
fn legacy_binary(path: &Path) -> Result<String> {
    let bytes = memory_map::read_file(path)?;
    let mut out = String::new();
    let mut run = String::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let u = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        let ch = char::from_u32(u32::from(u)).filter(|c| {
            (c.is_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation())
                && (u >= 0x20 || u == 0x0D || u == 0x0A || u == 0x09)
        });
        if let Some(c) = ch {
            run.push(if c == '\r' { '\n' } else { c });
            i += 2;
        } else {
            if run.chars().filter(|c| c.is_alphabetic()).count() >= 4 {
                out.push_str(&run);
                out.push('\n');
            }
            run.clear();
            i += if i % 2 == 0 { 2 } else { 1 };
        }
    }
    if run.chars().filter(|c| c.is_alphabetic()).count() >= 4 {
        out.push_str(&run);
    }
    run.clear();
    for &b in bytes.iter() {
        if (0x20..0x7F).contains(&b) || b == b'\r' || b == b'\n' || b == b'\t' {
            run.push(if b == b'\r' { '\n' } else { b as char });
        } else {
            if run.len() >= 8
                && run.chars().filter(char::is_ascii_alphabetic).count() * 2 > run.len()
                && run.contains(' ')
            {
                out.push_str(&run);
                out.push('\n');
            }
            run.clear();
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn extension_matches(extension: &OsStr, expected: &str) -> bool {
        extension
            .to_str()
            .is_some_and(|s| s.to_lowercase() == expected)
    }

    #[test]
    fn test_extension_matches() {
        assert!(extension_matches(OsStr::new("docx"), "docx"));
        assert!(extension_matches(OsStr::new("DOCX"), "docx"));
        assert!(extension_matches(OsStr::new("Docx"), "docx"));
        assert!(!extension_matches(OsStr::new("pdf"), "docx"));
    }

    #[tokio::test]
    async fn test_parse_file_txt() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "Hello, world!").unwrap();

        let result = parse_file(&file_path, false).await;
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.content.contains("Hello, world!"));
    }

    #[tokio::test]
    async fn test_parse_file_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.unknown");
        std::fs::File::create(&file_path).unwrap();

        let result = parse_file(&file_path, false).await;
        assert!(result.is_err());
    }
}
