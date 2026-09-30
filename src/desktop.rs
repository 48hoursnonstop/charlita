//! Small C ABI used by the Qt host. All editing stays in Rust; QML receives
//! serializable state and never sees Discord credentials.
use crate::{
    assets,
    engine::{Command, Engine},
    model::*,
    native::Native,
    output,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

struct Editor {
    draft: Document,
    undo: Vec<Document>,
    redo: Vec<Document>,
    busy: bool,
    message: String,
}
struct Shared {
    engine: Engine,
    editor: Mutex<Editor>,
}
impl Shared {
    fn change(&self, document: Document) -> Result<()> {
        document.validate()?;
        let mut editor = self.editor.lock().unwrap();
        if editor.draft == document {
            return Ok(());
        }
        self.engine.store.save("draft", &document)?;
        let previous = std::mem::replace(&mut editor.draft, document);
        editor.undo.push(previous);
        editor.redo.clear();
        if editor.undo.len() > 40 {
            editor.undo.remove(0);
        }
        drop(editor);
        self.engine.repaint();
        Ok(())
    }
    fn finish(&self, result: Result<String>) {
        let mut editor = self.editor.lock().unwrap();
        editor.busy = false;
        match result {
            Ok(message) => editor.message = message,
            Err(error) => {
                drop(editor);
                self.engine.report(error.to_string());
                return;
            }
        }
        drop(editor);
        self.engine.repaint();
    }
    fn snapshot(&self) -> Value {
        let editor = self.editor.lock().unwrap();
        let live = self.engine.live.read().unwrap();
        let runtime = self.engine.runtime.read().unwrap();
        let outputs: std::collections::BTreeMap<_, _> = editor
            .draft
            .profiles
            .iter()
            .flat_map(|p| &p.groups)
            .map(|g| {
                (
                    g.id.clone(),
                    output::group(&editor.draft, &runtime, g, &self.engine.key, true),
                )
            })
            .collect();
        let update = self.engine.updates.read().unwrap();
        let previews: std::collections::BTreeMap<_, _> = editor
            .draft
            .assets
            .values()
            .filter(|a| a.extension == "webm")
            .map(|a| (&a.id, assets::preview_path(&self.engine.store, a).is_file()))
            .collect();
        let commands: Vec<_> = self
            .engine
            .native_commands
            .lock()
            .unwrap()
            .drain(..)
            .map(|c| format!("{c:?}"))
            .collect();
        json!({"document":editor.draft,"dirty":editor.draft!=*live,"runtime":*runtime,"outputs":outputs,
            "busy":editor.busy,"message":editor.message,"previews":previews,"undo":!editor.undo.is_empty(),"redo":!editor.redo.is_empty(),
            "error":self.engine.notice.lock().unwrap().clone(),"assetRoot":self.engine.store.root.join("assets"),
            "root":self.engine.store.root,"baseUrl":format!("http://127.0.0.1:{}/o/{}",self.engine.port,self.engine.key),
            "redirect":format!("http://127.0.0.1:{}/auth/callback",self.engine.port),"commands":commands,
            "version":env!("CARGO_PKG_VERSION"),"update":{"status":update.status,"downloaded":update.downloaded,
                "url":update.release.as_ref().map(|r|&r.html_url),"version":update.release.as_ref().map(|r|&r.tag_name),
                "notes":update.release.as_ref().and_then(|r|r.body.as_ref())}})
    }
}
pub struct Desktop {
    shared: Arc<Shared>,
    native: Native,
    runtime: Option<tokio::runtime::Runtime>,
    _logs: tracing_appender::non_blocking::WorkerGuard,
}
impl Desktop {
    fn create(options: Value) -> Result<Self> {
        let explicit = options["data_dir"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        let root = crate::store::data_root(
            options["portable"].as_bool().unwrap_or(false),
            explicit.as_deref(),
        )?;
        let port = options["port"].as_u64().map(u16::try_from).transpose()?;
        let (engine, runtime) =
            match crate::service::start(&root, port, options["auto_check"].as_bool()) {
                Ok(result) => result,
                Err(error) => {
                    if crate::service::bring_existing_forward(&root).is_ok() {
                        bail!("already_running")
                    }
                    return Err(error);
                }
            };
        let (writer, logs) = tracing_appender::non_blocking(tracing_appender::rolling::daily(
            root.join("logs"),
            "charlita.log",
        ));
        engine.store.metadata(
            "portable_distribution",
            if crate::store::portable_distribution(options["portable"].as_bool().unwrap_or(false))?
            {
                "true"
            } else {
                "false"
            },
        )?;
        let _ = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(writer)
            .with_env_filter("charlita=info")
            .try_init();
        let native = Native::new(engine.clone());
        let draft = engine.store.load("draft")?.unwrap();
        let shared = Arc::new(Shared {
            engine,
            editor: Mutex::new(Editor {
                draft,
                undo: vec![],
                redo: vec![],
                busy: false,
                message: String::new(),
            }),
        });
        let missing: Vec<_> = shared
            .editor
            .lock()
            .unwrap()
            .draft
            .assets
            .values()
            .filter(|a| {
                a.extension == "webm" && !assets::preview_path(&shared.engine.store, a).is_file()
            })
            .cloned()
            .collect();
        if !missing.is_empty() {
            shared.editor.lock().unwrap().busy = true;
            let task = shared.clone();
            runtime.spawn_blocking(move || {
                let result = (|| {
                    for asset in missing {
                        assets::prepare_native_preview(&task.engine.store, &asset)?;
                    }
                    Ok(String::new())
                })();
                task.finish(result);
            });
        }
        Ok(Self {
            shared,
            native,
            runtime: Some(runtime),
            _logs: logs,
        })
    }
    fn request(&mut self, request: Value) -> Result<Value> {
        let shared = &self.shared;
        let engine = &shared.engine;
        let method = request["method"]
            .as_str()
            .context("Missing request method")?;
        let args = &request["args"];
        if matches!(
            method,
            "draft" | "apply" | "undo" | "redo" | "create" | "duplicate" | "add_guest" | "settings"
        ) {
            ensure!(
                !shared.editor.lock().unwrap().busy,
                "Wait for the current file operation / Espera a que termine la operación actual"
            );
        }
        match method {
            "snapshot" => return Ok(shared.snapshot()),
            "draft" => shared.change(serde_json::from_value(args["document"].clone())?)?,
            "apply" => {
                let document = shared.editor.lock().unwrap().draft.clone();
                let old = engine.live.read().unwrap().settings.clone();
                engine.apply(&document)?;
                if old.discord_app != document.settings.discord_app {
                    *engine.auth.lock().unwrap() = Default::default();
                }
                if old.discord_app != document.settings.discord_app
                    || old.pinned_channel != document.settings.pinned_channel
                {
                    let _ = engine.commands.send(Command::Connect);
                }
                if old.hotkeys != document.settings.hotkeys {
                    let _ = engine.commands.send(Command::RegisterHotkeys);
                }
                shared.editor.lock().unwrap().message = "Applied / Aplicado".into();
            }
            "undo" | "redo" => {
                let mut editor = shared.editor.lock().unwrap();
                let next = if method == "undo" {
                    editor.undo.pop()
                } else {
                    editor.redo.pop()
                };
                if let Some(next) = next {
                    engine.store.save("draft", &next)?;
                    let previous = std::mem::replace(&mut editor.draft, next);
                    if method == "undo" {
                        editor.redo.push(previous)
                    } else {
                        editor.undo.push(previous)
                    }
                }
                drop(editor);
                engine.repaint();
            }
            "create" => {
                let mut document = shared.editor.lock().unwrap().draft.clone();
                let name = text(args, "name")?;
                let created = match text(args, "kind")? {
                    "profile" => {
                        let p = Profile::new(name);
                        let id = p.id.clone();
                        document.profiles.push(p);
                        id
                    }
                    "group" => {
                        let g = Group::new(name);
                        let id = g.id.clone();
                        document
                            .profiles
                            .iter_mut()
                            .find(|p| p.id == args["profile"])
                            .context("Profile not found")?
                            .groups
                            .push(g);
                        id
                    }
                    "character" => {
                        let c = Character::new(name);
                        let id = c.id.clone();
                        document.characters.push(c);
                        id
                    }
                    _ => bail!("Unknown item kind"),
                };
                shared.change(document)?;
                return Ok(json!({"id":created}));
            }
            "duplicate" => {
                let mut document = shared.editor.lock().unwrap().draft.clone();
                let original = text(args, "id")?;
                let suffix = if document.settings.language == "es" {
                    " · copia"
                } else {
                    " · copy"
                };
                let created = match text(args, "kind")? {
                    "profile" => {
                        let mut profile = document
                            .profiles
                            .iter()
                            .find(|p| p.id == original)
                            .context("Profile not found")?
                            .clone();
                        profile.id = id();
                        profile.name.push_str(suffix);
                        for group in &mut profile.groups {
                            group.id = id();
                        }
                        let created = profile.id.clone();
                        document.profiles.push(profile);
                        created
                    }
                    "group" => {
                        let mut group =
                            document.group(original).context("Group not found")?.clone();
                        group.id = id();
                        group.name.push_str(suffix);
                        let created = group.id.clone();
                        document
                            .profiles
                            .iter_mut()
                            .find(|p| p.groups.iter().any(|g| g.id == original))
                            .unwrap()
                            .groups
                            .push(group);
                        created
                    }
                    "character" => {
                        let mut character = document
                            .characters
                            .iter()
                            .find(|c| c.id == original)
                            .context("Character not found")?
                            .clone();
                        character.id = id();
                        character.name.push_str(suffix);
                        let created = character.id.clone();
                        document.characters.push(character);
                        created
                    }
                    _ => bail!("Unknown item kind"),
                };
                shared.change(document)?;
                return Ok(json!({"id":created}));
            }
            "add_guest" => {
                let user = text(args, "user")?;
                ensure!(
                    !user.is_empty()
                        && user.len() <= 32
                        && user.chars().all(|c| c.is_ascii_digit()),
                    "Use the Discord user ID / Usa el ID del usuario de Discord"
                );
                let mut document = shared.editor.lock().unwrap().draft.clone();
                let discovered = engine.runtime.read().unwrap().discovered.get(user).cloned();
                document.people.entry(user.into()).or_insert_with(|| {
                    discovered.unwrap_or(Person {
                        id: user.into(),
                        name: args["name"].as_str().unwrap_or("Guest").into(),
                        avatar: None,
                        character: None,
                    })
                });
                let group = document
                    .group_mut(text(args, "group")?)
                    .context("Group not found")?;
                if !group.members.iter().any(|m| m.user == user) {
                    group.members.push(Member {
                        user: user.into(),
                        ..Default::default()
                    });
                }
                shared.change(document)?;
            }
            "live" => engine.live_action(text(args, "user")?, text(args, "action")?),
            "test" => engine.test(text(args, "user")?, text(args, "state")?),
            "settings" => {
                let draft = shared.editor.lock().unwrap().draft.clone();
                let mut live = engine.live.read().unwrap().clone();
                let reconnect = live.settings.discord_app != draft.settings.discord_app
                    || live.settings.pinned_channel != draft.settings.pinned_channel;
                if live.settings.discord_app != draft.settings.discord_app {
                    *engine.auth.lock().unwrap() = Default::default();
                }
                live.settings = draft.settings.clone();
                engine.store.save_pair(&draft, &live)?;
                *engine.live.write().unwrap() = live;
                engine.notify();
                if reconnect {
                    let _ = engine.commands.send(Command::Connect);
                }
            }
            "command" => {
                let command = match text(args, "name")? {
                    "connect" => Command::Connect,
                    "disconnect" => Command::Disconnect,
                    "authorize" => Command::Authorize,
                    "check_update" => Command::CheckUpdate,
                    "install_update" => Command::InstallUpdate,
                    _ => bail!("Unknown command"),
                };
                let _ = engine.commands.send(command);
            }
            "register_hotkeys" => {
                let _enter = self.runtime.as_ref().unwrap().enter();
                self.native.register()?;
            }
            "dismiss" => {
                *engine.notice.lock().unwrap() = None;
                shared.editor.lock().unwrap().message.clear();
            }
            "import_art" | "import_project" | "import_character" | "export_project"
            | "export_character" | "diagnostics" => {
                let mut editor = shared.editor.lock().unwrap();
                ensure!(
                    !editor.busy,
                    "Wait for the current file operation / Espera a que termine la operación actual"
                );
                editor.busy = true;
                drop(editor);
                let shared = shared.clone();
                let args = args.clone();
                let method = method.to_owned();
                self.runtime.as_ref().unwrap().spawn_blocking(move || {
                    let result = file_operation(&shared, &method, &args);
                    shared.finish(result);
                });
                engine.repaint();
            }
            _ => bail!("Unknown method: {method}"),
        }
        Ok(Value::Null)
    }
}
fn text<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key].as_str().with_context(|| format!("Missing {key}"))
}
fn file_operation(shared: &Shared, method: &str, args: &Value) -> Result<String> {
    let path = PathBuf::from(text(args, "path")?);
    let store = &shared.engine.store;
    let mut document = shared.editor.lock().unwrap().draft.clone();
    match method {
        "import_art" => {
            let asset = assets::import(store, &path)?;
            let character = args["character"].as_str().unwrap_or_default();
            let state = args["state"].as_str().unwrap_or("idle");
            if character.is_empty() {
                let mut c = Character::new(&asset.name);
                c.states.insert("idle".into(), Pose::new(asset.id.clone()));
                document.characters.push(c);
            } else {
                let c = document
                    .characters
                    .iter_mut()
                    .find(|c| c.id == character)
                    .context("Character no longer exists")?;
                if let Some(name) = state.strip_prefix("expression:") {
                    c.expressions
                        .insert(name.into(), Pose::new(asset.id.clone()));
                } else {
                    c.states.insert(state.into(), Pose::new(asset.id.clone()));
                }
            }
            document.assets.insert(asset.id.clone(), asset);
            shared.change(document)?;
        }
        "import_project" | "import_character" => {
            let imported = assets::import_package(store, &path, document.settings.clone())?;
            if method == "import_project" {
                document = imported;
            } else {
                for character in imported.characters {
                    if let Some(existing) = document
                        .characters
                        .iter_mut()
                        .find(|c| c.id == character.id)
                    {
                        *existing = character;
                    } else {
                        document.characters.push(character);
                    }
                }
                document.assets.extend(imported.assets);
            }
            shared.change(document)?;
        }
        "export_project" | "export_character" => {
            if method == "export_character" {
                let c = document
                    .characters
                    .iter()
                    .find(|c| c.id == args["character"])
                    .context("Select a character")?
                    .clone();
                let used: std::collections::BTreeSet<_> = c
                    .states
                    .values()
                    .chain(c.expressions.values())
                    .map(|p| p.asset.clone())
                    .collect();
                document.characters = vec![c];
                document.assets.retain(|id, _| used.contains(id));
                document.people.clear();
                document.profiles = vec![Profile::new("Character")];
            }
            assets::export_package(store, &document, &path)?;
        }
        "diagnostics" => {
            let r = shared.engine.runtime.read().unwrap();
            let info = json!({"version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"connection":r.connection,
                "people":r.users.len(),"profiles":document.profiles.len(),"characters":document.characters.len(),"assets":document.assets.len()});
            std::fs::write(path, serde_json::to_vec_pretty(&info)?)?;
        }
        _ => bail!("Unsupported file operation"),
    }
    Ok("Done / Listo".into())
}
impl Drop for Desktop {
    fn drop(&mut self) {
        *self.shared.engine.wake.write().unwrap() = None;
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_secs(3));
        }
    }
}
thread_local! {static LAST_ERROR:std::cell::RefCell<String>=const{std::cell::RefCell::new(String::new())};}
fn c_string(value: impl Into<String>) -> *mut c_char {
    CString::new(value.into().replace('\0', ""))
        .unwrap()
        .into_raw()
}
unsafe fn parse_json(pointer: *const c_char) -> Result<Value> {
    ensure!(!pointer.is_null(), "Null request");
    serde_json::from_slice(unsafe { CStr::from_ptr(pointer) }.to_bytes()).map_err(Into::into)
}
/// # Safety
/// options must point to a valid null-terminated UTF-8 JSON string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn charlita_create(options: *const c_char) -> *mut Desktop {
    let result = std::panic::catch_unwind(|| Desktop::create(unsafe { parse_json(options) }?));
    match result {
        Ok(Ok(desktop)) => Box::into_raw(Box::new(desktop)),
        other => {
            let message = match other {
                Ok(Err(error)) => format!("{error:#}"),
                _ => "Engine initialization failed".into(),
            };
            LAST_ERROR.with(|last| *last.borrow_mut() = message);
            std::ptr::null_mut()
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn charlita_last_error() -> *mut c_char {
    LAST_ERROR.with(|last| c_string(last.borrow().clone()))
}
/// # Safety
/// handle must be owned by the Qt thread and request must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn charlita_request(
    handle: *mut Desktop,
    request: *const c_char,
) -> *mut c_char {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        unsafe { handle.as_mut() }
            .context("No engine")?
            .request(unsafe { parse_json(request) }?)
    }));
    let value = match result {
        Ok(Ok(value)) => json!({"ok":true,"value":value}),
        Ok(Err(error)) => json!({"ok":false,"error":format!("{error:#}")}),
        _ => json!({"ok":false,"error":"Engine request failed"}),
    };
    c_string(value.to_string())
}
/// # Safety
/// The callback must remain valid until the desktop is destroyed. It may run
/// on worker threads and must queue work onto the Qt thread without blocking.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn charlita_set_wake(
    handle: *mut Desktop,
    context: *mut c_void,
    callback: extern "C" fn(*mut c_void),
) {
    if let Some(desktop) = unsafe { handle.as_ref() } {
        let context = context as usize;
        *desktop.shared.engine.wake.write().unwrap() =
            Some(Box::new(move || callback(context as *mut c_void)));
    }
}
/// # Safety
/// Must receive a pointer returned by charlita_create, exactly once, on its Qt thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn charlita_destroy(handle: *mut Desktop) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}
/// # Safety
/// Must receive a string returned by this module, exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn charlita_string_free(string: *mut c_char) {
    if !string.is_null() {
        drop(unsafe { CString::from_raw(string) });
    }
}
