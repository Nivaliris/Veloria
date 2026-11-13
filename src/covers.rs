//! Local cover cache. Never modifies the original book.
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}, process::Command, hash::{Hash, Hasher}, collections::hash_map::DefaultHasher, time::UNIX_EPOCH};

fn cache_dir() -> Option<PathBuf> {
    let p = dirs::cache_dir()?.join("Veloria").join("covers");
    fs::create_dir_all(&p).ok()?;
    Some(p)
}
fn key(path: &Path) -> String {
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    if let Ok(m) = fs::metadata(path) {
        m.len().hash(&mut h);
        if let Ok(t) = m.modified().and_then(|t|t.duration_since(UNIX_EPOCH).map_err(std::io::Error::other)) {
            t.as_secs().hash(&mut h);
        }
    }
    format!("{:016x}", h.finish())
}

pub fn cached(path: &Path) -> Option<PathBuf> {
    let dir = cache_dir()?;
    let stem = key(path);
    ["jpg", "jpeg", "png", "webp"].iter().map(|ext|dir.join(format!("{stem}.{ext}"))).find(|p|p.is_file())
}

fn embedded_epub(path: &Path) -> Option<PathBuf> {
    let dir = cache_dir()?;
    let file=fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    // Prefer recognized cover file names, avoiding decorative images in chapters.
    let mut images: Vec<(usize, i32, String)> = Vec::new();
    for i in 0..zip.len() {
        let Ok(entry) = zip.by_index(i) else {continue};
        let name=entry.name().to_string();
        let lower=name.to_lowercase();
        let ext=Path::new(&lower).extension().and_then(|s|s.to_str()).unwrap_or("");
        if !matches!(ext, "jpg"|"jpeg"|"png"|"webp") || entry.size() > 12_000_000 || entry.size() < 1_000 {continue}
        let score = if lower.contains("cover") {100} else if lower.contains("front") {80} else if lower.contains("titlepage") {50} else {0};
        images.push((i, score, ext.to_string()));
    }
    images.sort_by(|a,b| b.1.cmp(&a.1));
    let (i,score,ext)=images.into_iter().next()?;
    if score==0 {return None;} // Never select an arbitrary illustration as the cover.
    let mut entry=zip.by_index(i).ok()?;
    let target=dir.join(format!("{}.{}",key(path),ext));
    let tmp=target.with_extension("tmp");
    let mut bytes=Vec::new();
    entry.read_to_end(&mut bytes).ok()?;
    let mut out=fs::File::create(&tmp).ok()?;
    out.write_all(&bytes).ok()?;
    drop(out);
    fs::rename(tmp,&target).ok()?;
    Some(target)
}

#[cfg(target_os="macos")]
fn pdf_thumbnail(path: &Path) -> Option<PathBuf> {
    let dir=cache_dir()?;
    let staging=dir.join(format!("temp-{}",key(path)));
    fs::create_dir_all(&staging).ok()?;
    // QuickLook is built into macOS; return an image if the system can preview this PDF.
    let status=Command::new("/usr/bin/qlmanage")
        .args(["-t", "-s", "480", "-o"]).arg(&staging).arg(path)
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().ok()?;
    if !status.success() {let _=fs::remove_dir_all(&staging);return None;}
    let thumbnail=fs::read_dir(&staging).ok()?.flatten().map(|e|e.path())
        .find(|p|p.extension().and_then(|s|s.to_str())==Some("png"));
    let destination=dir.join(format!("{}.png",key(path)));
    let result=thumbnail.and_then(|p|fs::copy(&p,&destination).ok().map(|_|destination));
    let _=fs::remove_dir_all(&staging);
    result
}

/// Called only for visible books / selected details, not during the full directory scan.
pub fn resolve(path: &Path) -> Option<PathBuf> {
    if let Some(p)=cached(path) {return Some(p);}
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "epub" => embedded_epub(path),
        #[cfg(target_os="macos")]
        "pdf" => pdf_thumbnail(path),
        _ => None,
    }
}
