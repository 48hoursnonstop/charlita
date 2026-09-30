use crate::{
    engine::{Command, Engine},
    model::Binding,
};
use anyhow::{Context, Result, ensure};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

/// Main-thread resources: Windows tray and X11/Windows shortcut registration.
pub struct Native {
    engine: Engine,
    manager: Option<GlobalHotKeyManager>,
    keys: Vec<HotKey>,
    bindings: Arc<RwLock<BTreeMap<u32, Binding>>>,
    #[cfg(target_os = "linux")]
    portal: Option<tokio::task::JoinHandle<()>>,
    #[cfg(windows)]
    tray: Option<tray_icon::TrayIcon>,
}
impl Native {
    pub fn new(engine: Engine) -> Self {
        let bindings = Arc::new(RwLock::new(BTreeMap::<u32, Binding>::new()));
        let map = bindings.clone();
        let e = engine.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed {
                if let Some(binding) = map.read().unwrap().get(&event.id) {
                    e.live_action(&binding.user, &binding.action);
                }
            }
        }));
        #[cfg(windows)]
        let tray = windows_tray(&engine)
            .map_err(|err| engine.report(err.to_string()))
            .ok();
        Self {
            engine,
            manager: None,
            keys: vec![],
            bindings,
            #[cfg(target_os = "linux")]
            portal: None,
            #[cfg(windows)]
            tray,
        }
    }
    pub fn has_tray(&self) -> bool {
        #[cfg(windows)]
        {
            self.tray.is_some()
        }
        #[cfg(not(windows))]
        {
            false
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
            NewShortcut::new(&b.id, &format!("{name}: {}", b.action))
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

#[cfg(target_os = "linux")]
pub struct LinuxTray {
    engine: Engine,
}
#[cfg(target_os = "linux")]
impl ksni::Tray for LinuxTray {
    fn id(&self) -> String {
        "charlita".into()
    }
    fn title(&self) -> String {
        "Charlita".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let mut data = icon_rgba();
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }
        vec![ksni::Icon {
            width: 32,
            height: 32,
            data,
        }]
    }
    fn activate(&mut self, _: i32, _: i32) {
        let _ = self.engine.commands.send(Command::Show);
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let es = self.engine.live.read().unwrap().settings.language == "es";
        vec![
            ksni::menu::StandardItem {
                label: if es {
                    "Abrir Charlita"
                } else {
                    "Open Charlita"
                }
                .into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.engine.commands.send(Command::Show);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            ksni::menu::StandardItem {
                label: if es { "Salir" } else { "Quit" }.into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.engine.commands.send(Command::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
#[cfg(target_os = "linux")]
pub async fn linux_tray(engine: Engine) -> Result<ksni::Handle<LinuxTray>> {
    use ksni::TrayMethods;
    Ok(LinuxTray { engine }.spawn().await?)
}

#[cfg(windows)]
fn windows_tray(engine: &Engine) -> Result<tray_icon::TrayIcon> {
    use tray_icon::{
        Icon, TrayIconBuilder,
        menu::{Menu, MenuEvent, MenuItem},
    };
    let es = engine.live.read().unwrap().settings.language == "es";
    let menu = Menu::new();
    let show = MenuItem::new(
        if es {
            "Abrir Charlita"
        } else {
            "Open Charlita"
        },
        true,
        None,
    );
    let quit = MenuItem::new(if es { "Salir" } else { "Quit" }, true, None);
    menu.append(&show)?;
    menu.append(&quit)?;
    let show_id = show.id().clone();
    let quit_id = quit.id().clone();
    let e = engine.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == show_id {
            let _ = e.commands.send(Command::Show);
        } else if event.id == quit_id {
            let _ = e.commands.send(Command::Quit);
        }
    }));
    Ok(TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Charlita")
        .with_icon(Icon::from_rgba(icon_rgba(), 32, 32)?)
        .build()?)
}

pub fn icon_rgba() -> Vec<u8> {
    let mut data = vec![0; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let bubble = (4..28).contains(&x) && (5..24).contains(&y);
            let tail = (8..14).contains(&x) && (24..28).contains(&y) && x - 8 <= 27 - y;
            let eye = ((10..13).contains(&x) || (19..22).contains(&x)) && (12..17).contains(&y);
            let pixel = if eye {
                [21, 24, 29, 255]
            } else if bubble || tail {
                [126, 176, 230, 255]
            } else {
                [0, 0, 0, 0]
            };
            data[(y * 32 + x) * 4..(y * 32 + x + 1) * 4].copy_from_slice(&pixel);
        }
    }
    data
}
