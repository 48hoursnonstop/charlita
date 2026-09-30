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
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
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
    ensure!(
        fs::metadata(path)?.len() <= 1024 * 1024 * 1024,
        "Artwork larger than 1 GiB"
    );
    let (width, height) = inspect(path, &ext)?;
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
    if let Err(error) = prepare_native_preview(store, &a) {
        let _ = fs::remove_file(store.asset_path(&a));
        return Err(error);
    }
    Ok(a)
}
fn inspect(path: &Path, ext: &str) -> Result<(u32, u32)> {
    let (width, height) = if ext == "webm" {
        webm_decoder(path)?;
        let mut magic = [0; 4];
        File::open(path)?.read_exact(&mut magic)?;
        ensure!(magic == [0x1a, 0x45, 0xdf, 0xa3], "Invalid WebM header");
        let output = tool_output(
            Command::new(media_tool("ffprobe"))
                .args([
                    "-v",
                    "error",
                    "-select_streams",
                    "v:0",
                    "-show_entries",
                    "stream=width,height",
                    "-of",
                    "json",
                ])
                .arg(path),
        )
        .context(
            "Install or bundle FFmpeg (ffprobe) to import WebM / Instala FFmpeg para importar WebM",
        )?;
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
        let reader = ImageReader::open(path)?.with_guessed_format()?;
        let expected = match ext {
            "png" => image::ImageFormat::Png,
            "jpg" | "jpeg" => image::ImageFormat::Jpeg,
            "gif" => image::ImageFormat::Gif,
            "webp" => image::ImageFormat::WebP,
            _ => anyhow::bail!("Unsupported artwork format"),
        };
        ensure!(
            reader.format() == Some(expected),
            "The file extension does not match the artwork format / La extensión no corresponde al formato del archivo"
        );
        reader.into_dimensions()?
    };
    ensure!(
        width > 0 && height > 0 && width <= 16384 && height <= 16384,
        "Artwork dimensions must be 1–16384 px / Las dimensiones deben ser de 1–16384 px"
    );
    Ok((width, height))
}
// Run media tools off the UI thread, with bounded output and a deadline. Files
// keep pipe buffers from deadlocking while ffmpeg emits its first PNG frame.
fn tool_output(command: &mut Command) -> Result<Output> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW for background media tools.
    }
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline
            || stdout.metadata()?.len() > 16 * 1024 * 1024
            || stderr.metadata()?.len() > 1024 * 1024
        {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!(
                "Artwork could not be read within 30 seconds. Re-export it and import again / Vuelve a exportar el archivo e impórtalo de nuevo"
            );
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    use std::io::{Seek, SeekFrom};
    stdout.seek(SeekFrom::Start(0))?;
    stderr.seek(SeekFrom::Start(0))?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    stdout.take(16 * 1024 * 1024).read_to_end(&mut out)?;
    stderr.take(1024 * 1024).read_to_end(&mut err)?;
    Ok(Output {
        status,
        stdout: out,
        stderr: err,
    })
}
pub fn preview_path(store: &Store, asset: &Asset) -> PathBuf {
    store
        .root
        .join("assets")
        .join(format!("{}.preview.webp", asset.id))
}
pub fn prepare_native_preview(store: &Store, asset: &Asset) -> Result<()> {
    if asset.extension != "webm" || preview_path(store, asset).is_file() {
        return Ok(());
    }
    let temporary = tempfile::NamedTempFile::new_in(store.root.join("assets"))?;
    native_preview(&store.asset_path(asset), temporary.path())?;
    temporary.persist(preview_path(store, asset))?;
    Ok(())
}
// Qt's stock VP8/VP9 decoder drops WebM alpha. A bounded animated WebP is
// generated once for the native editor. Full-canvas frames avoid cropped-frame
// compositing artifacts in Qt. Stream output retains the original.
fn native_preview(source: &Path, destination: &Path) -> Result<()> {
    let decoder = webm_decoder(source)?;
    let result = tool_output(
        Command::new(media_tool("ffmpeg"))
            .args([
                "-v", "error", "-nostdin", "-y", "-threads", "2", "-c:v", decoder, "-i",
            ])
            .arg(source)
            .args([
                "-an",
                "-filter_threads",
                "1",
                "-vf",
                "fps=30,scale='min(512,iw)':'min(512,ih)':force_original_aspect_ratio=decrease",
                "-c:v",
                "libwebp",
                "-lossless",
                "0",
                "-q:v",
                "85",
                "-loop",
                "0",
                "-f",
                "webp",
            ])
            .arg(destination),
    )?;
    ensure!(
        result.status.success(),
        "Cannot prepare WebM preview. Re-export it as VP8/VP9 with alpha / Vuelve a exportar el WebM como VP8/VP9 con transparencia"
    );
    Ok(())
}
pub fn export_package(store: &Store, doc: &Document, path: &Path) -> Result<()> {
    doc.validate()?;
    let temp = tempfile::NamedTempFile::new_in(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
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
            inspect(&dest, &a.extension)? == (a.width, a.height),
            "Artwork dimensions do not match the character package"
        );
        ensure!(
            hash_file(&dest)? == a.sha256,
            "Asset checksum mismatch / El archivo del personaje está dañado"
        );
    }
    for a in doc.assets.values().filter(|a| a.extension == "webm") {
        native_preview(
            &stage.path().join(a.filename()),
            &stage.path().join(format!("{}.preview.webp", a.id)),
        )?;
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
        if a.extension == "webm" && !preview_path(store, a).exists() {
            fs::rename(
                stage.path().join(format!("{}.preview.webp", a.id)),
                preview_path(store, a),
            )?;
        }
    }
    Ok(doc)
}
pub fn thumbnail(store: &Store, a: &Asset) -> Result<image::RgbaImage> {
    let p = store.asset_path(a);
    if a.extension == "webm" {
        let decoder = webm_decoder(&p)?;
        let output = tool_output(
            Command::new(media_tool("ffmpeg"))
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
                ]),
        )?;
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
    let image = reader.decode()?;
    Ok(image
        .thumbnail(image.width().min(512), image.height().min(512))
        .into_rgba8())
}
pub fn webm_decoder(path: &Path) -> Result<&'static str> {
    let output = tool_output(
        Command::new(media_tool("ffprobe"))
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
            .arg(path),
    )?;
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
    fn imported_media_and_package_dimensions_must_match_the_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("app")).unwrap();
        let artwork = dir.path().join("fixture.png");
        image::RgbaImage::new(4, 4).save(&artwork).unwrap();
        let misleading = dir.path().join("fixture.gif");
        fs::copy(&artwork, &misleading).unwrap();
        assert!(import(&store, &misleading).is_err());
        let mut asset = import(&store, &artwork).unwrap();
        asset.width = 5;
        let mut document = Document::default();
        document.assets.insert(asset.id.clone(), asset);
        let package = dir.path().join("wrong-dimensions.charlita");
        export_package(&store, &document, &package).unwrap();
        assert!(import_package(&store, &package, document.settings).is_err());
    }
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
        broken.assets.get_mut(&a.id).unwrap().sha256 = "0".repeat(64);
        let bad = dir.path().join("bad.charlita");
        export_package(&s, &broken, &bad).unwrap();
        assert!(import_package(&s, &bad, d.settings).is_err());
    }
}
