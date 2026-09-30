#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
use anyhow::{Context, Result, bail};
use charlita::{
    engine::{Command, Engine},
    store::{Store, data_root},
};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    time::Duration,
};

fn main() {
    if let Err(error) = start() {
        eprintln!("Charlita: {error:#}");
        if !std::env::args().any(|a| a == "--headless") {
            rfd::MessageDialog::new()
                .set_title("Charlita")
                .set_description(format!("{error:#}"))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
        std::process::exit(1);
    }
}
fn start() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut portable = false;
    let mut headless = false;
    let mut explicit = None;
    let mut port = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--portable" => portable = true,
            "--headless" => headless = true,
            "--data-dir" => {
                explicit = Some(PathBuf::from(
                    args.next().context("--data-dir needs a path")?,
                ))
            }
            "--port" => {
                port = Some(
                    args.next()
                        .context("--port needs a number")?
                        .parse::<u16>()?,
                )
            }
            "--version" => {
                println!("Charlita {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!(
                    "Charlita [--portable] [--data-dir PATH] [--port PORT] [--headless]\nNative editor with local OBS/Streamlabs output."
                );
                return Ok(());
            }
            _ => bail!("Unknown option: {arg}"),
        }
    }
    let root = data_root(portable, explicit.as_deref())?;
    let store = match Store::open(root.clone()) {
        Ok(s) => Arc::new(s),
        Err(error) => {
            if bring_existing_forward(&root).is_ok() {
                return Ok(());
            }
            return Err(error);
        }
    };
    let appender = tracing_appender::rolling::daily(root.join("logs"), "charlita.log");
    let (writer, _log_guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(writer)
        .with_env_filter("charlita=info")
        .init();
    let (mut engine, commands) = Engine::new(store)?;
    if let Some(port) = port {
        let mut d = engine.live.read().unwrap().clone();
        d.settings.port = port;
        engine.apply(&d)?;
        engine.port = port;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let _enter = runtime.enter();
    let port = engine.live.read().unwrap().settings.port;
    let listener=runtime.block_on(tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,port)))
        .with_context(||format!("Cannot use local port {port}. Start Charlita with --port ANOTHER_PORT / El puerto está ocupado"))?;
    let server_engine = engine.clone();
    runtime.spawn(async move {
        if let Err(error) = charlita::server::serve(server_engine.clone(), listener).await {
            server_engine.report(format!("Overlay server: {error}"));
        }
    });
    let (native_tx, native_rx) = mpsc::channel();
    runtime.spawn(worker(engine.clone(), commands, native_tx.clone()));
    if engine.live.read().unwrap().settings.auto_check {
        let _ = engine.commands.send(Command::CheckUpdate);
    }
    if !engine.live.read().unwrap().settings.hotkeys.is_empty() {
        let _ = engine.commands.send(Command::RegisterHotkeys);
    }
    #[cfg(target_os = "linux")]
    let tray = if !headless {
        runtime.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(4),
                charlita::native::linux_tray(engine.clone()),
            )
            .await
            .ok()
            .and_then(Result::ok)
        })
    } else {
        None
    };
    #[cfg(target_os = "linux")]
    let tray_available = tray.is_some();
    #[cfg(not(target_os = "linux"))]
    let tray_available = false;
    tracing::info!(port, "Charlita started");
    if headless {
        println!("Charlita listening on http://127.0.0.1:{port}");
        runtime.block_on(tokio::signal::ctrl_c())?;
    } else {
        let icon = eframe::egui::IconData {
            rgba: charlita::native::icon_rgba(),
            width: 32,
            height: 32,
        };
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 800.0])
                .with_min_inner_size([960.0, 600.0])
                .with_app_id("io.github.48hoursnonstop.charlita")
                .with_icon(icon),
            ..Default::default()
        };
        eframe::run_native(
            "Charlita",
            options,
            Box::new(move |cc| {
                Ok(Box::new(charlita::gui::App::new(
                    cc,
                    engine,
                    native_rx,
                    native_tx,
                    tray_available,
                )))
            }),
        )
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    }
    #[cfg(target_os = "linux")]
    drop(tray);
    drop(_enter);
    runtime.shutdown_timeout(Duration::from_secs(3));
    Ok(())
}
async fn worker(
    engine: Engine,
    mut commands: tokio::sync::mpsc::UnboundedReceiver<Command>,
    native: mpsc::Sender<Command>,
) {
    let mut discord = tokio::spawn(charlita::discord::run(engine.clone()));
    let mut update_task: Option<tokio::task::JoinHandle<()>> = None;
    while let Some(command) = commands.recv().await {
        match command {
            Command::Connect => {
                discord.abort();
                discord = tokio::spawn(charlita::discord::run(engine.clone()));
            }
            Command::Disconnect => {
                discord.abort();
                charlita::discord::logout(&engine);
            }
            Command::Authorize => {
                let e = engine.clone();
                tokio::spawn(async move {
                    if let Err(error) = charlita::discord::authorize(e.clone()).await {
                        e.report(error.to_string());
                    }
                });
            }
            Command::CheckUpdate | Command::InstallUpdate => {
                if update_task.as_ref().is_some_and(|task| !task.is_finished()) {
                    continue;
                }
                let e = engine.clone();
                let install = matches!(command, Command::InstallUpdate);
                update_task = Some(tokio::spawn(async move {
                    let result = if install {
                        charlita::updates::install(e.clone()).await
                    } else {
                        charlita::updates::check(e.clone()).await
                    };
                    if let Err(error) = result {
                        charlita::updates::record_error(&e.updates, &error);
                        e.notify();
                    }
                }));
            }
            cmd => {
                let _ = native.send(cmd);
                engine.notify();
            }
        }
    }
}
fn bring_existing_forward(root: &std::path::Path) -> Result<()> {
    let db = rusqlite::Connection::open_with_flags(
        root.join("charlita.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let body: String = db.query_row("SELECT body FROM documents WHERE name='live'", [], |row| {
        row.get(0)
    })?;
    let doc: charlita::model::Document = serde_json::from_str(&body)?;
    let key: String = db.query_row(
        "SELECT value FROM metadata WHERE name='overlay_key'",
        [],
        |row| row.get(0),
    )?;
    // A separate capability is restricted to bringing this window forward.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        reqwest::Client::new()
            .post(format!(
                "http://127.0.0.1:{}/control/{key}/show",
                doc.settings.port
            ))
            .timeout(Duration::from_secs(2))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    })
}
