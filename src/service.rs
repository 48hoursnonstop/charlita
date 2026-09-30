use crate::{
    engine::{Command, Engine},
    store::Store,
};
use anyhow::{Context, Result};
use std::{path::Path, sync::Arc, time::Duration};

pub fn start(
    root: &Path,
    port: Option<u16>,
    auto_check: Option<bool>,
) -> Result<(Engine, tokio::runtime::Runtime)> {
    let store = Arc::new(Store::open(root.to_owned())?);
    if port.is_some() || auto_check.is_some() {
        let mut live = store.load("live")?.unwrap();
        let mut draft = store.load("draft")?.unwrap();
        if let Some(port) = port {
            live.settings.port = port;
            draft.settings.port = port;
        }
        if let Some(auto_check) = auto_check {
            live.settings.auto_check = auto_check;
            draft.settings.auto_check = auto_check;
        }
        store.save_pair(&draft, &live)?;
    }
    let (engine, commands) = Engine::new(store)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let listener=runtime.block_on(tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,engine.port)))
        .with_context(||format!("Local port {} is occupied. Restart with --port ANOTHER_PORT / El puerto está ocupado",engine.port))?;
    engine
        .store
        .metadata("listen_port", &engine.port.to_string())?;
    let e = engine.clone();
    runtime.spawn(async move {
        if let Err(error) = crate::server::serve(e.clone(), listener).await {
            e.report(format!("Overlay server: {error}"));
        }
    });
    runtime.spawn(worker(engine.clone(), commands));
    if engine.live.read().unwrap().settings.auto_check {
        let _ = engine.commands.send(Command::CheckUpdate);
    }
    Ok((engine, runtime))
}
async fn worker(engine: Engine, mut commands: tokio::sync::mpsc::UnboundedReceiver<Command>) {
    let mut discord = tokio::spawn(crate::discord::run(engine.clone()));
    let mut update_task: Option<tokio::task::JoinHandle<()>> = None;
    while let Some(command) = commands.recv().await {
        match command {
            Command::Connect => {
                discord.abort();
                discord = tokio::spawn(crate::discord::run(engine.clone()));
            }
            Command::Disconnect => {
                discord.abort();
                crate::discord::logout(&engine);
            }
            Command::Authorize => {
                let e = engine.clone();
                tokio::spawn(async move {
                    if let Err(error) = crate::discord::authorize(e.clone()).await {
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
                        crate::updates::install(e.clone()).await
                    } else {
                        crate::updates::check(e.clone()).await
                    };
                    if let Err(error) = result {
                        crate::updates::record_error(&e.updates, &error);
                        e.notify();
                    }
                }));
            }
            command => {
                engine.native_commands.lock().unwrap().push(command);
                engine.notify();
            }
        }
    }
}
pub fn bring_existing_forward(root: &Path) -> Result<()> {
    let db = rusqlite::Connection::open_with_flags(
        root.join("charlita.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let port: String = db.query_row(
        "SELECT value FROM metadata WHERE name='listen_port'",
        [],
        |r| r.get(0),
    )?;
    let key: String = db.query_row(
        "SELECT value FROM metadata WHERE name='overlay_key'",
        [],
        |r| r.get(0),
    )?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/control/{key}/show"))
            .timeout(Duration::from_secs(2))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    })
}
