use crate::Category;

const MIDIA: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "tif", "tiff", "raw", "cr2", "cr3",
    "nef", "arw", "dng", "svg", "psd", "mp4", "mkv", "mov", "avi", "wmv", "webm", "m4v", "flv",
    "mpg", "mpeg", "3gp", "ts", "mp3", "flac", "wav", "aac", "ogg", "opus", "m4a", "wma", "aiff",
];

const DOCUMENTOS: &[&str] = &[
    "pdf", "doc", "docx", "odt", "rtf", "txt", "md", "xls", "xlsx", "ods", "csv", "ppt", "pptx",
    "odp", "epub", "mobi", "azw3",
];

/// Categoria de um arquivo pela extensão (sem o ponto, qualquer caixa).
pub fn category_for_extension(ext: &str) -> Option<Category> {
    if ext.is_empty() || ext.len() > 8 {
        return None;
    }
    let ext = ext.to_ascii_lowercase();
    if MIDIA.contains(&ext.as_str()) {
        Some(Category::Midia)
    } else if DOCUMENTOS.contains(&ext.as_str()) {
        Some(Category::Documentos)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions() {
        assert_eq!(category_for_extension("MP4"), Some(Category::Midia));
        assert_eq!(category_for_extension("heic"), Some(Category::Midia));
        assert_eq!(category_for_extension("flac"), Some(Category::Midia));
        assert_eq!(category_for_extension("pdf"), Some(Category::Documentos));
        assert_eq!(category_for_extension("xlsx"), Some(Category::Documentos));
        assert_eq!(category_for_extension("exe"), None);
        assert_eq!(category_for_extension(""), None);
    }
}
