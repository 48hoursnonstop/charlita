//! Bounded native animation preview. Frames are decoded off the UI thread and
//! discarded when the preview cannot consume them; no entire clip is cached.
use crate::{assets::media_tool, model::Asset, store::Store};
use anyhow::{Context, Result, ensure};
use image::AnimationDecoder;
use std::{
    fs::File,
    io::{BufReader, Read},
    process::{Child, Command, Stdio},
    sync::{Arc, Condvar, Mutex, mpsc},
    time::Duration,
};

#[derive(Default)]
struct State {
    playing: bool,
    stop: bool,
}
pub struct Player {
    state: Arc<(Mutex<State>, Condvar)>,
    pub frames: mpsc::Receiver<Result<image::RgbaImage>>,
}
impl Player {
    pub fn new(store: Arc<Store>, asset: Asset, ctx: eframe::egui::Context) -> Self {
        let state = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let (tx, frames) = mpsc::sync_channel(2);
        let shared = state.clone();
        std::thread::spawn(move || {
            let run = || -> Result<()> {
                if asset.extension == "webm" {
                    return video(&store, &asset, &shared, &tx, &ctx);
                }
                loop {
                    if !wait(&shared, Duration::ZERO) {
                        return Ok(());
                    }
                    let file = BufReader::new(File::open(store.asset_path(&asset))?);
                    match asset.extension.as_str() {
                        "gif" => {
                            let mut decoder = image::codecs::gif::GifDecoder::new(file)?;
                            image::ImageDecoder::set_limits(&mut decoder, limits())?;
                            if !images(decoder.into_frames(), &shared, &tx, &ctx)? {
                                return Ok(());
                            }
                        }
                        "webp" => {
                            let mut decoder = image::codecs::webp::WebPDecoder::new(file)?;
                            image::ImageDecoder::set_limits(&mut decoder, limits())?;
                            if !decoder.has_animation() {
                                let img = image::ImageReader::open(store.asset_path(&asset))?
                                    .decode()?
                                    .thumbnail(512, 512)
                                    .into_rgba8();
                                emit(img, &tx, &ctx);
                                return Ok(());
                            }
                            if !images(decoder.into_frames(), &shared, &tx, &ctx)? {
                                return Ok(());
                            }
                        }
                        _ => return Ok(()),
                    }
                }
            };
            if let Err(error) = run() {
                let _ = tx.try_send(Err(error));
                ctx.request_repaint();
            }
        });
        Self { state, frames }
    }
    pub fn play(&self, playing: bool) {
        let (lock, wake) = &*self.state;
        let mut state = lock.lock().unwrap();
        if state.playing == playing {
            return;
        }
        state.playing = playing;
        wake.notify_all();
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        let (lock, wake) = &*self.state;
        lock.lock().unwrap().stop = true;
        wake.notify_all();
    }
}
fn limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits
}
type Shared = Arc<(Mutex<State>, Condvar)>;
fn wait(shared: &Shared, delay: Duration) -> bool {
    let (lock, wake) = &**shared;
    let mut state = lock.lock().unwrap();
    if !delay.is_zero() {
        state = wake.wait_timeout(state, delay).unwrap().0;
    }
    while !state.stop && !state.playing {
        state = wake.wait(state).unwrap();
    }
    !state.stop
}
fn emit(
    image: image::RgbaImage,
    tx: &mpsc::SyncSender<Result<image::RgbaImage>>,
    ctx: &eframe::egui::Context,
) {
    if tx.try_send(Ok(image)).is_ok() {
        ctx.request_repaint();
    }
}
fn images(
    frames: image::Frames<'_>,
    shared: &Shared,
    tx: &mpsc::SyncSender<Result<image::RgbaImage>>,
    ctx: &eframe::egui::Context,
) -> Result<bool> {
    let mut count = 0;
    for frame in frames {
        if !wait(shared, Duration::ZERO) {
            return Ok(false);
        }
        let frame = frame?;
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay = Duration::from_secs_f64(
            (numerator as f64 / denominator.max(1) as f64 / 1000.0).clamp(0.01, 60.0),
        );
        let image = image::DynamicImage::ImageRgba8(frame.into_buffer());
        let image = image.thumbnail(image.width().min(512),image.height().min(512)).into_rgba8();
        emit(image, tx, ctx);
        count += 1;
        if !wait(shared, delay) {
            return Ok(false);
        }
    }
    ensure!(count > 0, "Animation has no frames");
    Ok(true)
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn video(
    store: &Store,
    asset: &Asset,
    shared: &Shared,
    tx: &mpsc::SyncSender<Result<image::RgbaImage>>,
    ctx: &eframe::egui::Context,
) -> Result<()> {
    let (width, height) = if asset.width >= asset.height {
        (512, (512 * asset.height / asset.width).max(1))
    } else {
        ((512 * asset.width / asset.height).max(1), 512)
    };
    let codec = crate::assets::webm_decoder(&store.asset_path(asset))?;
    let mut child = Process(
        Command::new(media_tool("ffmpeg"))
            .args([
                "-v",
                "error",
                "-nostdin",
                "-stream_loop",
                "-1",
                "-c:v",
                codec,
                "-i",
            ])
            .arg(store.asset_path(asset))
            .args([
                "-an",
                "-vf",
                &format!("fps=30,scale={width}:{height}"),
                "-pix_fmt",
                "rgba",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Cannot start FFmpeg / No se pudo iniciar FFmpeg")?,
    );
    let mut stdout = child.0.stdout.take().unwrap();
    while wait(shared, Duration::ZERO) {
        let mut bytes = vec![0; (width * height * 4) as usize];
        stdout
            .read_exact(&mut bytes)
            .context("Cannot decode WebM animation")?;
        emit(
            image::RgbaImage::from_raw(width, height, bytes).unwrap(),
            tx,
            ctx,
        );
        if !wait(shared, Duration::from_millis(33)) {
            break;
        }
    }
    Ok(())
}
