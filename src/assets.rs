use crate::{
    model::{Asset, Document, id},
    store::Store,
};
use anyhow::{Context, Result, ensure};
use image::ImageReader;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

pub fn media_tool(name: &str) -> PathBuf {
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    };
    let exe = std::env::current_exe().unwrap_or_default();
    for p in [
        exe.with_file_name(&filename),
        exe.parent()
            .unwrap_or(Path::new("."))
            .join("tools")
            .join(&filename),
        PathBuf::from("tools").join(&filename),
    ] {
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from(filename)
}
pub fn hash_file(p: &Path) -> Result<String> {
    let mut f = File::open(p)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
pub fn import(store: &Store, path: &Path) -> Result<Asset> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    ensure!(
        matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "webm"
        ),
        "Import PNG, JPEG, WebP, GIF or WebM / Importa PNG, JPEG, WebP, GIF o WebM"
    );
    let (width, height) = if ext == "webm" {
        let mut magic = [0; 4];
        File::open(path)?.read_exact(&mut magic)?;
        ensure!(magic == [0x1a, 0x45, 0xdf, 0xa3], "Invalid WebM header");
        let output=Command::new(media_tool("ffprobe")).args(["-v","error","-select_streams","v:0","-show_entries","stream=width,height","-of","json"]).arg(path).output().context("Install or bundle FFmpeg (ffprobe) to import WebM / Instala FFmpeg para importar WebM")?;
        ensure!(
            output.status.success(),
            "Cannot read WebM; export a VP8/VP9 WebM file"
        );
        let v: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        (
            v["streams"][0]["width"]
                .as_u64()
                .context("WebM has no video")? as u32,
            v["streams"][0]["height"]
                .as_u64()
                .context("WebM has no video")? as u32,
        )
    } else {
        ImageReader::open(path)?
            .with_guessed_format()?
            .into_dimensions()?
    };
    ensure!(
        width > 0 && height > 0 && width <= 16384 && height <= 16384,
        "Artwork dimensions must be 1–16384 px / Las dimensiones deben ser de 1–16384 px"
    );
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "video/webm",
    };
    let a = Asset {
        id: id(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        extension: ext,
        mime: mime.into(),
        width,
        height,
        sha256: hash_file(path)?,
    };
    fs::copy(path, store.asset_path(&a))?;
    Ok(a)
}
pub fn export_package(store: &Store, doc: &Document, path: &Path) -> Result<()> {
    doc.validate()?;
    let temp = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))?;
    let mut zip = zip::ZipWriter::new(temp.reopen()?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut clean = doc.clone();
    clean.settings = Default::default();
    zip.start_file("manifest.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(&clean)?)?;
    for a in clean.assets.values() {
        zip.start_file(format!("assets/{}", a.filename()), options)?;
        std::io::copy(&mut File::open(store.asset_path(a))?, &mut zip)?;
    }
    zip.finish()?.sync_all()?;
    temp.persist(path)?;
    Ok(())
}
pub fn import_package(
    store: &Store,
    path: &Path,
    settings: crate::model::Settings,
) -> Result<Document> {
    let mut zip = zip::ZipArchive::new(File::open(path)?)?;
    let mut body = String::new();
    zip.by_name("manifest.json")?
        .take(16 * 1024 * 1024)
        .read_to_string(&mut body)?;
    let mut doc: Document = serde_json::from_str(&body)?;
    doc.settings = settings;
    doc.validate()?;
    let stage = tempfile::tempdir_in(&store.root)?;
    // Only manifest-declared, validated UUID filenames are extracted; archive paths are never trusted.
    for a in doc.assets.values() {
        let filename = a.filename();
        let mut entry = zip.by_name(&format!("assets/{filename}"))?;
        ensure!(
            entry.size() <= 1024 * 1024 * 1024,
            "Asset larger than 1 GiB"
        );
        let dest = stage.path().join(&filename);
        std::io::copy(&mut entry, &mut File::create(&dest)?)?;
        ensure!(
            hash_file(&dest)? == a.sha256,
            "Asset checksum mismatch / El archivo del personaje está dañado"
        );
    }
    for a in doc.assets.values() {
        let dst = store.asset_path(a);
        if dst.exists() {
            ensure!(
                hash_file(&dst)? == a.sha256,
                "An imported asset ID conflicts with an existing file"
            );
        } else {
            fs::rename(stage.path().join(a.filename()), dst)?;
        }
    }
    Ok(doc)
}
pub fn thumbnail(store: &Store, a: &Asset) -> Result<image::RgbaImage> {
    let p = store.asset_path(a);
    if a.extension == "webm" {
        let decoder = webm_decoder(&p)?;
        let output = Command::new(media_tool("ffmpeg"))
            .args(["-v", "error", "-nostdin", "-c:v", decoder, "-i"])
            .arg(&p)
            .args([
                "-frames:v",
                "1",
                "-vf",
                "scale='min(512,iw)':'min(512,ih)':force_original_aspect_ratio=decrease",
                "-f",
                "image2pipe",
                "-vcodec",
                "png",
                "-",
            ])
            .output()?;
        ensure!(
            output.status.success(),
            "Cannot preview WebM. Verify FFmpeg is installed"
        );
        return Ok(image::load_from_memory(&output.stdout)?.into_rgba8());
    }
    let mut reader = ImageReader::open(p)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let image=reader.decode()?;
    Ok(image.thumbnail(image.width().min(512), image.height().min(512)).into_rgba8())
}
pub fn webm_decoder(path: &Path) -> Result<&'static str> {
    let output = Command::new(media_tool("ffprobe"))
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()?;
    ensure!(output.status.success(), "Cannot inspect WebM codec");
    match String::from_utf8_lossy(&output.stdout).trim() {
        "vp8" => Ok("libvpx"),
        "vp9" => Ok("libvpx-vp9"),
        _ => anyhow::bail!("WebM artwork must use VP8 or VP9 / El WebM debe usar VP8 o VP9"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_roundtrip_and_corruption_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path().join("app")).unwrap();
        let image_path = dir.path().join("face.png");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([100, 100, 100, 0]))
            .save(&image_path)
            .unwrap();
        let a = import(&s, &image_path).unwrap();
        let mut d = Document::default();
        d.assets.insert(a.id.clone(), a.clone());
        let p = dir.path().join("pack.charlita");
        export_package(&s, &d, &p).unwrap();
        let result = import_package(&s, &p, d.settings.clone()).unwrap();
        assert_eq!(result.assets, d.assets);
        let mut broken = d.clone();
        broken.assets.get_mut(&a.id).unwrap().sha256 = "incorrect".into();
        let bad = dir.path().join("bad.charlita");
        export_package(&s, &broken, &bad).unwrap();
        assert!(import_package(&s, &bad, d.settings).is_err());
    }
}
