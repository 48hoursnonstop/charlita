use crate::{engine::Engine, model::Binding};
use anyhow::{Context, Result, ensure};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

/// Main-thread resources for X11/Windows shortcut registration.
pub struct Native {
    engine: Engine,
    manager: Option<GlobalHotKeyManager>,
    keys: Vec<HotKey>,
    bindings: Arc<RwLock<BTreeMap<u32, Binding>>>,
    #[cfg(target_os = "linux")]
    portal: Option<tokio::task::JoinHandle<()>>,
}
impl Native {
    pub fn new(engine: Engine) -> Self {
        let bindings = Arc::new(RwLock::new(BTreeMap::<u32, Binding>::new()));
        let map = bindings.clone();
        let e = engine.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed
                && let Some(binding) = map.read().unwrap().get(&event.id)
            {
                e.live_action(&binding.user, &binding.action);
            }
        }));
        Self {
            engine,
            manager: None,
            keys: vec![],
            bindings,
            #[cfg(target_os = "linux")]
            portal: None,
        }
    }
    pub fn register(&mut self) -> Result<()> {
        let bindings = self.engine.live.read().unwrap().settings.hotkeys.clone();
        let mut parsed = BTreeMap::new();
        let mut keys = Vec::new();
        for binding in &bindings {
            let key: HotKey = binding.shortcut.parse().with_context(|| {
                format!("Invalid shortcut: {} / Atajo inválido", binding.shortcut)
            })?;
            ensure!(
                !binding.user.is_empty(),
                "Choose a guest for every shortcut / Selecciona un invitado para cada atajo"
            );
            ensure!(
                parsed.insert(key.id(), binding.clone()).is_none(),
                "Duplicate shortcut / Atajo duplicado"
            );
            keys.push(key);
        }
        #[cfg(target_os = "linux")]
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            if let Some(task) = self.portal.take() {
                task.abort();
            }
            if !bindings.is_empty() {
                let e = self.engine.clone();
                self.portal = Some(tokio::spawn(async move {
                    if let Err(error) = portal_shortcuts(e.clone(), bindings).await {
                        e.report(format!("Global shortcuts: {error}. Your desktop must provide the GlobalShortcuts portal / El escritorio debe ofrecer el portal GlobalShortcuts"));
                    }
                }));
            }
            return Ok(());
        }
        if self.manager.is_none() && !keys.is_empty() {
            self.manager = Some(GlobalHotKeyManager::new()?);
        }
        if let Some(manager) = &self.manager {
            manager.unregister_all(&self.keys)?;
            self.keys.clear();
            self.bindings.write().unwrap().clear();
            for key in &keys {
                if let Err(error) = manager.register(*key) {
                    let _ = manager.unregister_all(&self.keys);
                    self.keys.clear();
                    return Err(error.into());
                }
                self.keys.push(*key);
            }
            *self.bindings.write().unwrap() = parsed;
        }
        Ok(())
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        if let Some(manager) = &self.manager {
            let _ = manager.unregister_all(&self.keys);
        }
        #[cfg(target_os = "linux")]
        if let Some(task) = self.portal.take() {
            task.abort();
        }
        GlobalHotKeyEvent::set_event_handler(None::<fn(GlobalHotKeyEvent)>);
    }
}

#[cfg(target_os = "linux")]
async fn portal_shortcuts(engine: Engine, bindings: Vec<Binding>) -> Result<()> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use futures_util::StreamExt;
    let portal = GlobalShortcuts::new().await?;
    let session = portal.create_session(Default::default()).await?;
    // The portal owns consent and may let the user choose a different key combination.
    let triggers: Vec<_> = bindings
        .iter()
        .map(|b| portal_trigger(&b.shortcut))
        .collect();
    let shortcuts: Vec<_> = bindings
        .iter()
        .zip(&triggers)
        .map(|(b, trigger)| {
            let name = engine
                .live
                .read()
                .unwrap()
                .people
                .get(&b.user)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            NewShortcut::new(&b.id, format!("{name}: {}", b.action))
                .preferred_trigger(Some(trigger.as_str()))
        })
        .collect();
    let mut activated = portal.receive_activated().await?;
    portal
        .bind_shortcuts(&session, &shortcuts, None, Default::default())
        .await?
        .response()?;
    while let Some(event) = activated.next().await {
        if let Some(b) = bindings.iter().find(|b| b.id == event.shortcut_id()) {
            engine.live_action(&b.user, &b.action);
        }
    }
    Ok(())
}
#[cfg(target_os = "linux")]
fn portal_trigger(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(|key| match key.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "CTRL".into(),
            "alt" => "ALT".into(),
            "shift" => "SHIFT".into(),
            "super" | "meta" | "win" => "LOGO".into(),
            _ => key.trim().to_owned(),
        })
        .collect::<Vec<String>>()
        .join("+")
}
