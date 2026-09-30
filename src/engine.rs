use crate::{
    model::{Document, Presence, Runtime},
    store::Store,
};
use anyhow::Result;
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, mpsc};

type WakeCallback = Box<dyn Fn() + Send + Sync>;
#[derive(Clone)]
pub struct Engine {
    pub store: Arc<Store>,
    pub live: Arc<RwLock<Document>>,
    pub runtime: Arc<RwLock<Runtime>>,
    pub events: broadcast::Sender<()>,
    pub wake: Arc<RwLock<Option<WakeCallback>>>,
    pub commands: mpsc::UnboundedSender<Command>,
    pub key: String,
    pub port: u16,
    pub auth: Arc<std::sync::Mutex<crate::discord::AuthState>>,
    pub updates: Arc<RwLock<crate::updates::UpdateState>>,
    pub notice: Arc<std::sync::Mutex<Option<String>>>,
    pub native_commands: Arc<std::sync::Mutex<Vec<Command>>>,
}
#[derive(Clone, Debug)]
pub enum Command {
    Connect,
    Disconnect,
    Authorize,
    Show,
    Quit,
    CheckUpdate,
    InstallUpdate,
    RegisterHotkeys,
}
impl Engine {
    pub fn new(store: Arc<Store>) -> Result<(Self, mpsc::UnboundedReceiver<Command>)> {
        let live = store.load("live")?.unwrap();
        let (events, _) = broadcast::channel(32);
        let (commands, rx) = mpsc::unbounded_channel();
        let key = store.token()?;
        let port = live.settings.port;
        Ok((
            Self {
                store,
                live: Arc::new(RwLock::new(live)),
                runtime: Arc::new(RwLock::new(Runtime {
                    connection: "not_configured".into(),
                    ..Default::default()
                })),
                events,
                wake: Default::default(),
                commands,
                key,
                port,
                auth: Default::default(),
                updates: Default::default(),
                notice: Default::default(),
                native_commands: Default::default(),
            },
            rx,
        ))
    }
    pub fn notify(&self) {
        let _ = self.events.send(());
        self.repaint();
    }
    pub fn repaint(&self) {
        if let Some(wake) = self.wake.read().unwrap().as_ref() {
            wake();
        }
    }
    pub fn report(&self, message: impl Into<String>) {
        *self.notice.lock().unwrap() = Some(message.into());
        self.notify();
    }
    pub fn apply(&self, d: &Document) -> Result<()> {
        self.store.apply(d)?;
        *self.live.write().unwrap() = d.clone();
        self.notify();
        Ok(())
    }
    pub fn connection(&self, status: &str) {
        let mut r = self.runtime.write().unwrap();
        if r.connection == status {
            return;
        }
        r.connection = status.into();
        if status != "connected" {
            for p in r.users.values_mut() {
                p.speaking = false;
            }
        }
        drop(r);
        self.notify();
    }
    pub fn live_action(&self, user: &str, action: &str) {
        let mut r = self.runtime.write().unwrap();
        let p = r.users.entry(user.into()).or_default();
        match action {
            "toggle" => p.hidden = Some(!p.hidden.unwrap_or(false)),
            "show" => p.hidden = Some(false),
            "hide" => p.hidden = Some(true),
            "reset" => p.expression = None,
            name => p.expression = Some(name.into()),
        }
        drop(r);
        self.notify();
    }
    pub fn test(&self, user: &str, state: &str) {
        let mut r = self.runtime.write().unwrap();
        if state == "off" {
            r.test.remove(user);
        } else {
            r.test.insert(
                user.into(),
                Presence {
                    speaking: state == "speaking",
                    muted: state == "muted",
                    present: state != "absent",
                    ..Default::default()
                },
            );
        }
        drop(r);
        if let Some(wake) = self.wake.read().unwrap().as_ref() {
            wake();
        }
    }
}
