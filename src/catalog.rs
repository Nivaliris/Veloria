use std::{fs, io::Read, path::{Path, PathBuf}, time::SystemTime};
use walkdir::WalkDir;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Book {
    pub path: PathBuf,
    pub title: String,
    pub author: String,
    pub format: String,
    pub cover: Option<PathBuf>,
    pub modified: SystemTime,
    pub favorite: bool,
    pub status: String,
    pub categories: Vec<String>,
}

pub fn supported_extension(ext: &str) -> bool {
    matches!(ext, "epub" | "pdf" | "mobi" | "azw3" | "cbz" | "txt")
}

fn normal_title(stem: &str) -> String {
    stem.replace(['_', '.'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn epub_meta(path: &Path) -> Option<(String, String)> {
    let file = fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let re_title = Regex::new(r"(?s)<(?:dc:)?title(?:\s[^>]*)?>(.*?)</(?:dc:)?title>").ok()?;
    let re_author = Regex::new(r"(?s)<(?:dc:)?creator(?:\s[^>]*)?>(.*?)</(?:dc:)?creator>").ok()?;
    let mut result = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).ok()?;
        if !entry.name().ends_with(".opf") { continue; }
        let mut xml = String::new();
        if entry.take(1024 * 1024).read_to_string(&mut xml).is_err() { continue; }
        let title = re_title.captures(&xml).map(|m| xml_decode(&m[1]));
        let author = re_author.captures(&xml).map(|m| xml_decode(&m[1]));
        if let (Some(t), Some(a)) = (title, author) { result = Some((t, a)); break; }
    }
    result
}

fn xml_decode(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'")
}

fn find_cover(folder: &Path, stem: &str) -> Option<PathBuf> {
    for name in ["cover", "folder", "front", stem] {
        for ext in ["jpg", "jpeg", "png", "webp"] {
            let candidate = folder.join(format!("{name}.{ext}"));
            if candidate.is_file() { return Some(candidate); }
        }
    }
    None
}

pub fn scan(root: &Path) -> Vec<Book> {
    let mut books = Vec::new();
    if !root.is_dir() { return books; }
    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() { continue; }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if !supported_extension(&ext) { continue; }
        let parent = path.parent().unwrap_or(root);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Sem título");
        let parent_rel = parent.strip_prefix(root).unwrap_or(parent);
        let mut parts = parent_rel.components();
        let author_from_folder = parts.next().map(|s| s.as_os_str().to_string_lossy().into_owned());
        let mut title = normal_title(stem);
        let mut author = author_from_folder.unwrap_or_else(|| "Autor desconhecido".into());
        if ext == "epub" {
            if let Some((t, a)) = epub_meta(path) {
                if !t.trim().is_empty() { title = t; }
                if !a.trim().is_empty() { author = a; }
            }
        }
        let modified = entry.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(SystemTime::UNIX_EPOCH);
        books.push(Book {
            path: path.to_path_buf(), title, author, format: ext.to_uppercase(),
            cover: find_cover(parent, stem), modified, favorite: false,
            status: "Por ler".to_string(), categories: vec![],
        });
    }
    books.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    books
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn formats() { assert!(supported_extension("epub")); assert!(!supported_extension("exe")); }
    #[test] fn names() { assert_eq!(normal_title("my_book.name"), "my book name"); }
}
