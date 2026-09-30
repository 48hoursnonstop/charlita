use crate::{
    assets,
    engine::{Command, Engine},
    model::*,
    output,
};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, TextureHandle, Vec2};
use std::{
    collections::BTreeMap,
    sync::mpsc,
    time::{Duration, Instant},
};

const BG: Color32 = Color32::from_rgb(21, 24, 29);
const SURFACE: Color32 = Color32::from_rgb(28, 32, 39);
const TEXT: Color32 = Color32::from_rgb(233, 237, 244);
const MUTED: Color32 = Color32::from_rgb(162, 173, 190);
const ACCENT: Color32 = Color32::from_rgb(126, 176, 230);
const ERROR: Color32 = Color32::from_rgb(248, 149, 154);

enum Work {
    Asset(String, String, anyhow::Result<Asset>),
    Package(bool, anyhow::Result<Document>),
    Thumb(String, anyhow::Result<image::RgbaImage>),
    Message(anyhow::Result<String>),
}
struct Thumb {
    texture: Option<TextureHandle>,
    error: Option<String>,
    last: Instant,
}
struct Animated {
    player: crate::media::Player,
    texture: Option<TextureHandle>,
    last: Instant,
}
pub struct App {
    engine: Engine,
    draft: Document,
    profile: usize,
    group: String,
    selected: Option<String>,
    character: Option<String>,
    view: u8,
    rx: mpsc::Receiver<Work>,
    tx: mpsc::Sender<Work>,
    thumbs: BTreeMap<String, Thumb>,
    animated: BTreeMap<String, Animated>,
    dirty: bool,
    last_edit: Instant,
    undo: Vec<Document>,
    redo: Vec<Document>,
    message: Option<String>,
    error: Option<String>,
    busy: usize,
    expression_name: String,
    new_name: String,
    new_id: String,
    delete: Option<(String, String)>,
    quit: bool,
    tray_available: bool,
    native_rx: mpsc::Receiver<Command>,
    pub native_tx: mpsc::Sender<Command>,
    native: crate::native::Native,
}
impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        engine: Engine,
        native_rx: mpsc::Receiver<Command>,
        native_tx: mpsc::Sender<Command>,
        tray_available: bool,
    ) -> Self {
        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = SURFACE;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.widgets.inactive.bg_fill = SURFACE;
        style.visuals.selection.bg_fill = Color32::from_rgb(42, 66, 95);
        style.visuals.selection.stroke = Stroke::new(1.5, ACCENT);
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.interact_size = Vec2::new(40.0, 32.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
        *engine.ui.write().unwrap() = Some(cc.egui_ctx.clone());
        let draft = engine.store.load("draft").unwrap().unwrap();
        let group = draft.profiles[0].groups[0].id.clone();
        let dirty = draft != *engine.live.read().unwrap();
        let (tx, rx) = mpsc::channel();
        let native = crate::native::Native::new(engine.clone());
        let tray_available = tray_available || native.has_tray();
        Self {
            engine,
            draft,
            profile: 0,
            group,
            selected: None,
            character: None,
            view: 0,
            rx,
            tx,
            thumbs: BTreeMap::new(),
            animated: BTreeMap::new(),
            dirty,
            last_edit: Instant::now(),
            undo: vec![],
            redo: vec![],
            message: None,
            error: None,
            busy: 0,
            expression_name: String::new(),
            new_name: String::new(),
            new_id: String::new(),
            delete: None,
            quit: false,
            tray_available,
            native_rx,
            native_tx,
            native,
        }
    }
    fn es(&self) -> bool {
        self.draft.settings.language == "es"
    }
    fn t<'a>(&self, es: &'a str, en: &'a str) -> &'a str {
        if self.es() { es } else { en }
    }
    fn result(&mut self, result: anyhow::Result<String>) {
        match result {
            Ok(msg) => {
                self.message = Some(msg);
                self.error = None
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    fn url(&self, kind: &str, id: &str) -> String {
        format!(
            "http://127.0.0.1:{}/o/{}/{kind}/{id}",
            self.engine.port, self.engine.key
        )
    }
    fn apply(&mut self) {
        let r = self.engine.apply(&self.draft);
        match r {
            Ok(()) => {
                self.dirty = false;
                self.message = Some(
                    self.t(
                        "Cambios aplicados al overlay",
                        "Changes applied to the overlay",
                    )
                    .into(),
                )
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    fn import_art(&mut self, char_id: String, state: String, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let store = self.engine.store.clone();
        let ctx = ctx.clone();
        self.busy += 1;
        std::thread::spawn(move || {
            let r = if let Some(path) = rfd::FileDialog::new()
                .add_filter("Artwork", &["png", "jpg", "jpeg", "webp", "gif", "webm"])
                .pick_file()
            {
                assets::import(&store, &path)
            } else {
                Err(anyhow::anyhow!("cancelled"))
            };
            let _ = tx.send(Work::Asset(char_id, state, r));
            ctx.request_repaint();
        });
    }
    fn package(&mut self, import: bool, character_only: bool, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let store = self.engine.store.clone();
        let ctx = ctx.clone();
        let mut doc = self.draft.clone();
        if character_only && !import {
            let Some(id) = self.character.clone() else {
                return;
            };
            let Some(c) = doc.characters.iter().find(|c| c.id == id).cloned() else {
                return;
            };
            let used: std::collections::BTreeSet<_> = c
                .states
                .values()
                .chain(c.expressions.values())
                .map(|p| p.asset.clone())
                .collect();
            doc.characters = vec![c];
            doc.assets.retain(|id, _| used.contains(id));
            doc.people.clear();
            doc.profiles = vec![Profile::new("Character")];
        }
        self.busy += 1;
        std::thread::spawn(move || {
            let mut dialog = rfd::FileDialog::new().add_filter("Charlita", &["charlita"]);
            if !import {
                dialog = dialog.set_file_name("charlita.charlita");
            }
            if import {
                let r = dialog
                    .pick_file()
                    .map(|p| assets::import_package(&store, &p, doc.settings))
                    .unwrap_or_else(|| Err(anyhow::anyhow!("cancelled")));
                let _ = tx.send(Work::Package(character_only, r));
            } else {
                let r = dialog
                    .save_file()
                    .map(|p| {
                        assets::export_package(&store, &doc, &p)
                            .map(|_| "Exported / Exportado".into())
                    })
                    .unwrap_or_else(|| Err(anyhow::anyhow!("cancelled")));
                let _ = tx.send(Work::Message(r));
            }
            ctx.request_repaint();
        });
    }
    fn process(&mut self, ctx: &egui::Context) {
        while let Ok(work) = self.rx.try_recv() {
            match work {
                Work::Asset(char_id, state, result) => {
                    self.busy = self.busy.saturating_sub(1);
                    match result {
                        Ok(a) => {
                            let aid = a.id.clone();
                            if char_id.is_empty() {
                                let mut c = Character::new(&a.name);
                                c.states.insert("idle".into(), Pose::new(aid.clone()));
                                self.character = Some(c.id.clone());
                                self.draft.characters.push(c);
                                self.view = 1;
                            } else if let Some(c) =
                                self.draft.characters.iter_mut().find(|c| c.id == char_id)
                            {
                                if let Some(name) = state.strip_prefix("expression:") {
                                    c.expressions.insert(name.into(), Pose::new(aid.clone()));
                                } else {
                                    c.states.insert(state, Pose::new(aid.clone()));
                                }
                            }
                            self.draft.assets.insert(aid, a);
                            self.dirty = true;
                            self.last_edit = Instant::now();
                        }
                        Err(e) => {
                            if e.to_string() != "cancelled" {
                                self.error = Some(e.to_string())
                            }
                        }
                    }
                }
                Work::Package(characters, result) => {
                    self.busy = self.busy.saturating_sub(1);
                    match result {
                        Ok(doc) => {
                            self.undo.push(self.draft.clone());
                            if characters {
                                for c in doc.characters {
                                    if let Some(existing) =
                                        self.draft.characters.iter_mut().find(|old| old.id == c.id)
                                    {
                                        *existing = c
                                    } else {
                                        self.draft.characters.push(c)
                                    }
                                }
                                self.draft.assets.extend(doc.assets);
                            } else {
                                self.draft = doc;
                                self.profile = 0;
                                self.group = self.draft.profiles[0].groups[0].id.clone();
                                self.selected = None;
                            }
                            self.dirty = true;
                            self.last_edit = Instant::now();
                            self.message = Some(
                                self.t(
                                    "Importado en el borrador. Revisa y aplica los cambios.",
                                    "Imported into the draft. Review and apply your changes.",
                                )
                                .into(),
                            );
                        }
                        Err(e) => {
                            if e.to_string() != "cancelled" {
                                self.error = Some(e.to_string())
                            }
                        }
                    }
                }
                Work::Thumb(id, result) => {
                    if let Some(t) = self.thumbs.get_mut(&id) {
                        match result {
                            Ok(img) => {
                                t.texture = Some(ctx.load_texture(
                                    &id,
                                    egui::ColorImage::from_rgba_unmultiplied(
                                        [img.width() as usize, img.height() as usize],
                                        img.as_raw(),
                                    ),
                                    egui::TextureOptions::LINEAR,
                                ))
                            }
                            Err(e) => t.error = Some(e.to_string()),
                        }
                    }
                }
                Work::Message(r) => {
                    self.busy = self.busy.saturating_sub(1);
                    if !r.as_ref().is_err_and(|e| e.to_string() == "cancelled") {
                        self.result(r)
                    }
                }
            }
        }
        self.thumbs
            .retain(|_, t| t.last.elapsed() < Duration::from_secs(30));
        self.animated
            .retain(|_, t| t.last.elapsed() < Duration::from_secs(30));
    }
    fn animated_texture(&mut self, id: &str, ctx: &egui::Context) -> Option<TextureHandle> {
        let asset = self.draft.assets.get(id)?.clone();
        if self.draft.settings.reduced_motion
            || !matches!(asset.extension.as_str(), "gif" | "webp" | "webm")
        {
            return self.texture(id, ctx);
        }
        let animation = self.animated.entry(id.into()).or_insert_with(|| Animated {
            player: crate::media::Player::new(self.engine.store.clone(), asset, ctx.clone()),
            texture: None,
            last: Instant::now(),
        });
        animation.last = Instant::now();
        animation.player.play(true);
        while let Ok(frame) = animation.player.frames.try_recv() {
            match frame {
                Ok(image) => {
                    let color = egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as usize, image.height() as usize],
                        image.as_raw(),
                    );
                    if let Some(texture) = &mut animation.texture {
                        texture.set(color, egui::TextureOptions::LINEAR);
                    } else {
                        animation.texture =
                            Some(ctx.load_texture(id, color, egui::TextureOptions::LINEAR));
                    }
                }
                Err(error) => self.error = Some(format!("Animation preview: {error}")),
            }
        }
        let texture = animation.texture.clone();
        texture.or_else(|| self.texture(id, ctx))
    }
    fn texture(&mut self, id: &str, ctx: &egui::Context) -> Option<TextureHandle> {
        if let Some(t) = self.thumbs.get_mut(id) {
            t.last = Instant::now();
            return t.texture.clone();
        }
        let a = self.draft.assets.get(id)?.clone();
        let store = self.engine.store.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let aid = id.to_owned();
        self.thumbs.insert(
            id.into(),
            Thumb {
                texture: None,
                error: None,
                last: Instant::now(),
            },
        );
        std::thread::spawn(move || {
            let result = assets::thumbnail(&store, &a);
            let _ = tx.send(Work::Thumb(aid, result));
            ctx.request_repaint();
        });
        None
    }
    fn header(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(16))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Charlita").size(23.0).strong());
                    ui.add_space(16.0);
                    for (view, es, en) in [
                        (0, "Grupos", "Groups"),
                        (1, "Personajes", "Characters"),
                        (2, "Ajustes", "Settings"),
                    ] {
                        if ui
                            .selectable_label(self.view == view, self.t(es, en))
                            .clicked()
                        {
                            self.view = view;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(self.t("Aplicar", "Apply")).color(BG),
                                )
                                .fill(ACCENT),
                            )
                            .clicked()
                        {
                            self.apply();
                        }
                        ui.label(
                            RichText::new(if self.dirty {
                                self.t("Borrador sin publicar", "Unpublished draft")
                            } else {
                                self.t("Publicado", "Published")
                            })
                            .color(MUTED),
                        );
                        if ui.button(self.t("Deshacer", "Undo")).clicked() {
                            self.undo();
                        }
                        if ui.button(self.t("Rehacer", "Redo")).clicked() {
                            self.redo();
                        }
                    });
                });
            });
    }
    fn undo(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(self.draft.clone());
            self.draft = d;
            self.dirty = true;
            self.normalize_selection();
        }
    }
    fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(self.draft.clone());
            self.draft = d;
            self.dirty = true;
            self.normalize_selection();
        }
    }
    fn normalize_selection(&mut self) {
        self.profile = self.profile.min(self.draft.profiles.len() - 1);
        if !self.draft.profiles[self.profile]
            .groups
            .iter()
            .any(|g| g.id == self.group)
        {
            self.group = self.draft.profiles[self.profile].groups[0].id.clone();
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|id| !self.draft.people.contains_key(id))
        {
            self.selected = None
        }
    }
    fn status(&self) -> String {
        let r = self.engine.runtime.read().unwrap();
        match r.connection.as_str() {
            "connected" => format!(
                "{} · {}",
                self.t("Discord conectado", "Discord connected"),
                r.channel_name
            ),
            "not_configured" => self
                .t(
                    "Configura Discord en Ajustes",
                    "Configure Discord in Settings",
                )
                .into(),
            "authorization_required" | "signed_out" => self
                .t(
                    "Autoriza Discord en Ajustes",
                    "Authorize Discord in Settings",
                )
                .into(),
            "connecting" => self
                .t("Conectando con Discord…", "Connecting to Discord…")
                .into(),
            "reconnecting" => self
                .t(
                    "Reconectando · personajes en reposo",
                    "Reconnecting · characters at rest",
                )
                .into(),
            "authorizing" => self
                .t(
                    "Completa la autorización en Discord",
                    "Complete authorization in Discord",
                )
                .into(),
            _ => self
                .t(
                    "Revisa la conexión en Ajustes",
                    "Check the connection in Settings",
                )
                .into(),
        }
    }
    fn footer(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("footer")
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(12))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.status());
                    if self.busy > 0 {
                        ui.spinner();
                        ui.label(self.t("Procesando archivos…", "Processing files…"));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(self.t("Salir", "Quit")).clicked() {
                            self.quit = true;
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if self.tray_available
                            && ui
                                .button(self.t("Ocultar ventana", "Hide window"))
                                .clicked()
                        {
                            for animation in self.animated.values() {
                                animation.player.play(false);
                            }
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        }
                    });
                });
                if let Some(error) = self.error.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(ERROR, error);
                        if ui.button(self.t("Cerrar aviso", "Dismiss")).clicked() {
                            self.error = None;
                        }
                    });
                } else if let Some(msg) = self.message.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(msg);
                        if ui.button(self.t("Cerrar aviso", "Dismiss")).clicked() {
                            self.message = None;
                        }
                    });
                }
            });
    }
    fn groups(&mut self, ui: &mut egui::Ui) {
        let es = self.es();
        egui::Panel::left("navigation")
            .default_size(270.0)
            .size_range(220.0..=390.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(16))
            .show(ui, |ui| {
                ui.heading(self.t("Perfiles", "Profiles"));
                let profiles: Vec<_> = self
                    .draft
                    .profiles
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i, p.name.clone()))
                    .collect();
                egui::ComboBox::from_id_salt("profile")
                    .selected_text(&self.draft.profiles[self.profile].name)
                    .show_ui(ui, |ui| {
                        for (i, name) in profiles {
                            if ui.selectable_value(&mut self.profile, i, name).clicked() {
                                self.group = self.draft.profiles[i].groups[0].id.clone();
                                self.selected = None;
                            }
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(self.t("Crear perfil", "Create profile"))
                        .clicked()
                    {
                        self.draft.profiles.push(Profile::new(if es {
                            "Nuevo perfil"
                        } else {
                            "New profile"
                        }));
                        self.profile = self.draft.profiles.len() - 1;
                        self.group = self.draft.profiles[self.profile].groups[0].id.clone();
                    }
                    if ui.button(self.t("Duplicar", "Duplicate")).clicked() {
                        let mut p = self.draft.profiles[self.profile].clone();
                        p.id = id();
                        p.name.push_str(" · copy");
                        for g in &mut p.groups {
                            g.id = id()
                        }
                        self.draft.profiles.push(p);
                    }
                    if self.draft.profiles.len() > 1
                        && ui
                            .button(self.t("Eliminar perfil", "Delete profile"))
                            .clicked()
                    {
                        self.delete = Some((
                            "profile".into(),
                            self.draft.profiles[self.profile].id.clone(),
                        ));
                    }
                });
                ui.label(self.t("Nombre del perfil", "Profile name"));
                ui.text_edit_singleline(&mut self.draft.profiles[self.profile].name);
                ui.add_space(16.0);
                ui.heading(self.t("Grupos", "Groups"));
                for g in self.draft.profiles[self.profile].groups.clone() {
                    if ui.selectable_label(g.id == self.group, g.name).clicked() {
                        self.group = g.id;
                        self.selected = None;
                    }
                }
                if ui.button(self.t("Crear grupo", "Create group")).clicked() {
                    let g = Group::new(if es { "Nuevo grupo" } else { "New group" });
                    self.group = g.id.clone();
                    self.draft.profiles[self.profile].groups.push(g);
                }
                ui.add_space(18.0);
                ui.heading(self.t("Invitados", "Guests"));
                let runtime = self.engine.runtime.read().unwrap().clone();
                let members = self.draft.group(&self.group).unwrap().members.clone();
                egui::ScrollArea::vertical()
                    .id_salt("guests")
                    .max_height(ui.available_height() - 120.0)
                    .show(ui, |ui| {
                        for m in members {
                            ui.push_id(&m.user, |ui| {
                                let name = self
                                    .draft
                                    .people
                                    .get(&m.user)
                                    .map(|p| p.name.as_str())
                                    .unwrap_or("Guest");
                                if ui
                                    .selectable_label(
                                        self.selected.as_deref() == Some(&m.user),
                                        name,
                                    )
                                    .clicked()
                                {
                                    self.selected = Some(m.user.clone());
                                }
                                let state = runtime.users.get(&m.user);
                                let status = if state.is_some_and(|p| p.muted) {
                                    if es { "Silenciado" } else { "Muted" }
                                } else if state.is_some_and(|p| p.speaking) {
                                    if es { "Hablando" } else { "Speaking" }
                                } else if state.is_some_and(|p| p.present) {
                                    if es { "En llamada" } else { "In call" }
                                } else {
                                    if es { "Ausente" } else { "Absent" }
                                };
                                ui.label(RichText::new(status).small().color(MUTED));
                            });
                        }
                        for p in runtime
                            .discovered
                            .values()
                            .filter(|p| {
                                !self
                                    .draft
                                    .group(&self.group)
                                    .unwrap()
                                    .members
                                    .iter()
                                    .any(|m| m.user == p.id)
                            })
                            .cloned()
                            .collect::<Vec<_>>()
                        {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(&p.name);
                                if ui
                                    .button(if es {
                                        "Añadir al grupo"
                                    } else {
                                        "Add to group"
                                    })
                                    .clicked()
                                {
                                    self.draft.people.entry(p.id.clone()).or_insert(p.clone());
                                    self.draft.group_mut(&self.group).unwrap().members.push(
                                        Member {
                                            user: p.id.clone(),
                                            ..Default::default()
                                        },
                                    );
                                    self.selected = Some(p.id);
                                }
                            });
                        }
                    });
                egui::CollapsingHeader::new(self.t("Añadir por ID", "Add by ID")).show(ui, |ui| {
                    ui.label(if es { "Nombre" } else { "Name" });
                    ui.text_edit_singleline(&mut self.new_name);
                    ui.label("Discord ID");
                    ui.text_edit_singleline(&mut self.new_id);
                    if ui
                        .button(if es { "Añadir invitado" } else { "Add guest" })
                        .clicked()
                    {
                        if self.new_id.len() < 5
                            || !self.new_id.chars().all(|c| c.is_ascii_digit())
                            || self.new_name.trim().is_empty()
                        {
                            self.error = Some(
                                if es {
                                    "Introduce un nombre y el ID numérico de Discord"
                                } else {
                                    "Enter a name and numeric Discord ID"
                                }
                                .into(),
                            );
                        } else {
                            let uid = self.new_id.clone();
                            self.draft.people.entry(uid.clone()).or_insert(Person {
                                id: uid.clone(),
                                name: self.new_name.clone(),
                                character: None,
                                avatar: None,
                            });
                            if !self
                                .draft
                                .group(&self.group)
                                .unwrap()
                                .members
                                .iter()
                                .any(|m| m.user == uid)
                            {
                                self.draft
                                    .group_mut(&self.group)
                                    .unwrap()
                                    .members
                                    .push(Member {
                                        user: uid.clone(),
                                        ..Default::default()
                                    });
                            }
                            self.selected = Some(uid);
                            self.new_id.clear();
                            self.new_name.clear();
                        }
                    }
                });
            });
        egui::Panel::right("properties")
            .default_size(300.0)
            .size_range(260.0..=400.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(16))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.group_properties(ui);
                    if self.selected.is_some() {
                        ui.add_space(20.0);
                        self.guest_properties(ui);
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(24))
            .show(ui, |ui| {
                ui.heading(self.t("Vista previa", "Preview"));
                ui.label(
                    RichText::new(self.t(
                        "Los cambios llegan al stream al pulsar Aplicar.",
                        "Changes reach the stream when you select Apply.",
                    ))
                    .color(MUTED),
                );
                ui.add_space(16.0);
                self.canvas(ui);
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(self.t("Copiar URL del grupo", "Copy group URL"))
                        .clicked()
                    {
                        ui.ctx().copy_text(self.url("group", &self.group));
                    }
                    if ui
                        .button(self.t("Abrir salida publicada", "Open published output"))
                        .clicked()
                    {
                        let _ = open::that(self.url("group", &self.group));
                    }
                    if ui
                        .button(self.t("Importar proyecto", "Import project"))
                        .clicked()
                    {
                        self.package(true, false, ui.ctx());
                    }
                    if ui
                        .button(self.t("Exportar proyecto", "Export project"))
                        .clicked()
                    {
                        self.package(false, false, ui.ctx());
                    }
                });
            });
    }
    fn group_properties(&mut self, ui: &mut egui::Ui) {
        let es = self.es();
        ui.heading(self.t("Composición", "Composition"));
        let group_id = self.group.clone();
        let can_delete = self.draft.profiles[self.profile].groups.len() > 1;
        let g = self.draft.group_mut(&group_id).unwrap();
        ui.label(if es { "Nombre del grupo" } else { "Group name" });
        ui.text_edit_singleline(&mut g.name);
        egui::ComboBox::from_id_salt("layout")
            .selected_text(layout_name(g.layout, es))
            .show_ui(ui, |ui| {
                for mode in [Layout::Row, Layout::Column, Layout::Grid, Layout::Free] {
                    ui.selectable_value(&mut g.layout, mode, layout_name(mode, es));
                }
            });
        ui.horizontal(|ui| {
            ui.label(if es { "Ancho" } else { "Width" });
            ui.add(egui::DragValue::new(&mut g.width).range(64..=8192));
            ui.label(if es { "Alto" } else { "Height" });
            ui.add(egui::DragValue::new(&mut g.height).range(64..=8192));
        });
        ui.add(egui::Slider::new(&mut g.gap, 0.0..=200.0).text(if es {
            "Separación"
        } else {
            "Spacing"
        }));
        if g.layout == Layout::Grid {
            ui.add(
                egui::DragValue::new(&mut g.columns)
                    .range(1..=100)
                    .prefix(if es { "Columnas: " } else { "Columns: " }),
            );
        }
        ui.checkbox(
            &mut g.preserve_spaces,
            if es {
                "Conservar espacios"
            } else {
                "Preserve spaces"
            },
        );
        ui.checkbox(
            &mut g.labels,
            if es { "Mostrar nombres" } else { "Show names" },
        );
        let mut duplicate = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if es {
                    "Duplicar grupo"
                } else {
                    "Duplicate group"
                })
                .clicked()
            {
                let mut clone = g.clone();
                clone.id = id();
                clone.name.push_str(" · copy");
                duplicate = Some(clone);
            }
            if can_delete
                && ui
                    .button(if es { "Eliminar grupo" } else { "Delete group" })
                    .clicked()
            {
                self.delete = Some(("group".into(), group_id));
            }
        });
        if let Some(group) = duplicate {
            self.draft.profiles[self.profile].groups.push(group);
        }
    }
    fn guest_properties(&mut self, ui: &mut egui::Ui) {
        let es = self.es();
        let uid = self.selected.clone().unwrap();
        ui.heading(if es {
            "Invitado seleccionado"
        } else {
            "Selected guest"
        });
        let characters = self.draft.characters.clone();
        let person = self.draft.people.get_mut(&uid).unwrap();
        ui.label(if es { "Nombre visible" } else { "Display name" });
        ui.text_edit_singleline(&mut person.name);
        character_picker(
            ui,
            "usual",
            if es {
                "Personaje habitual"
            } else {
                "Usual character"
            },
            &mut person.character,
            &characters,
            es,
        );
        let g = self.draft.group_mut(&self.group).unwrap();
        let index = g.members.iter().position(|m| m.user == uid).unwrap();
        let m = &mut g.members[index];
        character_picker(
            ui,
            "override",
            if es {
                "Personaje en este grupo"
            } else {
                "Character in this group"
            },
            &mut m.character,
            &characters,
            es,
        );
        ui.checkbox(
            &mut m.enabled,
            if es {
                "Incluir en la composición"
            } else {
                "Include in composition"
            },
        );
        ui.checkbox(
            &mut m.mirror,
            if es {
                "Voltear horizontalmente"
            } else {
                "Flip horizontally"
            },
        );
        ui.checkbox(
            &mut m.locked,
            if es {
                "Bloquear posición"
            } else {
                "Lock position"
            },
        );
        ui.add(egui::Slider::new(&mut m.size, 16.0..=1000.0).text(if es {
            "Tamaño"
        } else {
            "Size"
        }));
        if g.layout == Layout::Free {
            ui.horizontal(|ui| {
                ui.label("X");
                ui.add(egui::DragValue::new(&mut m.x).speed(1.0));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut m.y).speed(1.0));
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button(if es { "Centrar" } else { "Center" }).clicked() {
                    m.x = (g.width as f32 - m.size) / 2.0;
                    m.y = (g.height as f32 - m.size) / 2.0;
                }
                if ui
                    .button(if es { "Alinear abajo" } else { "Align bottom" })
                    .clicked()
                {
                    m.y = (g.height as f32 - m.size - 40.0).max(0.0);
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if es {
                    "Traer al frente"
                } else {
                    "Bring forward"
                })
                .clicked()
                && index + 1 < g.members.len()
            {
                g.members.swap(index, index + 1);
            }
            if ui
                .button(if es { "Enviar atrás" } else { "Send backward" })
                .clicked()
                && index > 0
            {
                g.members.swap(index, index - 1);
            }
            if ui
                .button(if es {
                    "Quitar del grupo"
                } else {
                    "Remove from group"
                })
                .clicked()
            {
                g.members.retain(|m| m.user != uid);
                self.selected = None;
            }
        });
        ui.add_space(16.0);
        ui.label(
            RichText::new(if es {
                "Control del directo"
            } else {
                "Live controls"
            })
            .strong(),
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if es { "Mostrar ahora" } else { "Show now" })
                .clicked()
            {
                self.engine.live_action(&uid, "show");
            }
            if ui
                .button(if es { "Ocultar ahora" } else { "Hide now" })
                .clicked()
            {
                self.engine.live_action(&uid, "hide");
            }
        });
        let expression_names = self
            .draft
            .group(&self.group)
            .and_then(|g| g.members.iter().find(|m| m.user == uid))
            .and_then(|m| self.draft.character_for(m))
            .map(|c| c.expressions.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for name in expression_names {
            if ui.button(&name).clicked() {
                self.engine.live_action(&uid, &name);
            }
        }
        if ui
            .button(if es {
                "Restablecer expresión"
            } else {
                "Reset expression"
            })
            .clicked()
        {
            self.engine.live_action(&uid, "reset");
        }
        if ui
            .button(if es {
                "Copiar URL individual"
            } else {
                "Copy individual URL"
            })
            .clicked()
        {
            ui.ctx().copy_text(self.url("person", &uid));
        }
        ui.add_space(16.0);
        ui.label(
            RichText::new(if es {
                "Simular en vista previa"
            } else {
                "Simulate in preview"
            })
            .strong(),
        );
        ui.horizontal_wrapped(|ui| {
            for (state, label) in [
                ("idle", if es { "Reposo" } else { "Idle" }),
                ("speaking", if es { "Habla" } else { "Speaking" }),
                ("muted", if es { "Mute" } else { "Muted" }),
                ("absent", if es { "Ausente" } else { "Absent" }),
                ("off", if es { "Terminar prueba" } else { "Stop test" }),
            ] {
                if ui.button(label).clicked() {
                    self.engine.test(&uid, state);
                }
            }
        });
    }
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let g = self.draft.group(&self.group).unwrap().clone();
        let runtime = self.engine.runtime.read().unwrap().clone();
        let out = output::group(&self.draft, &runtime, &g, &self.engine.key, true);
        let size = Vec2::new(
            ui.available_width(),
            ui.available_height().max(160.0) - 100.0,
        );
        let scale = (size.x / g.width as f32)
            .min(size.y / g.height as f32)
            .max(0.01);
        let dims = Vec2::new(g.width as f32 * scale, g.height as f32 * scale);
        let (rect, response) = ui.allocate_exact_size(dims, egui::Sense::click());
        let painter = ui.painter_at(rect);
        let checker = 16.0;
        for row in 0..=(dims.y / checker) as usize {
            for col in 0..=(dims.x / checker) as usize {
                let cell = Rect::from_min_size(
                    rect.min + Vec2::new(col as f32 * checker, row as f32 * checker),
                    Vec2::splat(checker),
                )
                .intersect(rect);
                painter.rect_filled(
                    cell,
                    0.0,
                    if (row + col) % 2 == 0 {
                        Color32::from_rgb(31, 35, 42)
                    } else {
                        Color32::from_rgb(36, 41, 49)
                    },
                );
            }
        }
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(66, 75, 91)),
            StrokeKind::Inside,
        );
        if out.guests.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                self.t(
                    "Añade invitados para componer el grupo",
                    "Add guests to compose this group",
                ),
                egui::FontId::proportional(17.0),
                MUTED,
            );
        }
        for guest in out.guests.into_iter().filter(|g| g.visible) {
            let jump = if guest.state == "speaking" && !self.draft.settings.reduced_motion {
                guest.effects.jump
            } else {
                0.0
            };
            let actor = Rect::from_min_size(
                rect.min + Vec2::new(guest.x, guest.y - jump) * scale,
                Vec2::splat(guest.size * scale),
            );
            let response = ui.interact(
                actor,
                ui.id().with(&guest.id),
                egui::Sense::click_and_drag(),
            );
            if response.clicked() {
                self.selected = Some(guest.id.clone());
                response.request_focus();
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &guest.name)
            });
            if response.dragged() && g.layout == Layout::Free {
                let delta = ui.input(|i| i.pointer.delta()) / scale;
                if let Some(m) = self
                    .draft
                    .group_mut(&self.group)
                    .unwrap()
                    .members
                    .iter_mut()
                    .find(|m| m.user == guest.id)
                {
                    if !m.locked {
                        m.x += delta.x;
                        m.y += delta.y;
                    }
                }
            }
            let texture = guest
                .pose
                .as_ref()
                .and_then(|pose| self.animated_texture(&pose.asset, ui.ctx()));
            if let Some(texture) = texture {
                let pose = guest.pose.as_ref().unwrap();
                let mut crop = pose.crop;
                if let Some(sprite) = &pose.sprite {
                    let time = ui.input(|i| i.time) as f32;
                    let frame = if self.draft.settings.reduced_motion {
                        0
                    } else {
                        (time * sprite.fps) as u32 % sprite.frames
                    };
                    crop = [
                        (frame % sprite.columns) as f32 / sprite.columns as f32
                            + crop[0] / sprite.columns as f32,
                        (frame / sprite.columns) as f32 / sprite.rows as f32
                            + crop[1] / sprite.rows as f32,
                        crop[2] / sprite.columns as f32,
                        crop[3] / sprite.rows as f32,
                    ];
                    if !self.draft.settings.reduced_motion {
                        ui.ctx().request_repaint_after(Duration::from_millis(
                            (1000.0 / sprite.fps) as u64,
                        ));
                    }
                }
                let mut uv =
                    Rect::from_min_size(Pos2::new(crop[0], crop[1]), Vec2::new(crop[2], crop[3]));
                if guest.mirror {
                    std::mem::swap(&mut uv.min.x, &mut uv.max.x);
                }
                let aspect = texture.size_vec2().x / texture.size_vec2().y * crop[2] / crop[3];
                let drawn = if aspect >= 1.0 {
                    Rect::from_center_size(
                        actor.center(),
                        Vec2::new(actor.width(), actor.height() / aspect),
                    )
                } else {
                    Rect::from_center_size(
                        actor.center(),
                        Vec2::new(actor.width() * aspect, actor.height()),
                    )
                };
                painter.image(texture.id(), drawn, uv, Color32::WHITE);
            } else {
                painter.circle_filled(
                    actor.center(),
                    actor.width() / 2.0,
                    Color32::from_rgb(60, 71, 86),
                );
                painter.text(
                    actor.center(),
                    egui::Align2::CENTER_CENTER,
                    guest.name.chars().take(2).collect::<String>(),
                    egui::FontId::proportional((actor.width() * 0.2).clamp(12.0, 48.0)),
                    TEXT,
                );
            }
            if guest.state == "speaking" {
                painter.rect_stroke(actor, 8.0, Stroke::new(2.0, ACCENT), StrokeKind::Outside);
            }
            if self.selected.as_deref() == Some(&guest.id) {
                painter.rect_stroke(
                    actor,
                    4.0,
                    Stroke::new(1.5, Color32::WHITE),
                    StrokeKind::Outside,
                );
            }
            if out.labels {
                painter.text(
                    actor.center_bottom() + Vec2::new(0.0, 14.0),
                    egui::Align2::CENTER_CENTER,
                    &guest.name,
                    egui::FontId::proportional((18.0 * scale).max(12.0)),
                    TEXT,
                );
            }
        }
        if let Some(uid) = self.selected.clone() {
            let delta = ui.input(|i| {
                let speed = if i.modifiers.shift { 10.0 } else { 1.0 };
                Vec2::new(
                    (if i.key_pressed(egui::Key::ArrowRight) {
                        speed
                    } else {
                        0.0
                    }) - (if i.key_pressed(egui::Key::ArrowLeft) {
                        speed
                    } else {
                        0.0
                    }),
                    (if i.key_pressed(egui::Key::ArrowDown) {
                        speed
                    } else {
                        0.0
                    }) - (if i.key_pressed(egui::Key::ArrowUp) {
                        speed
                    } else {
                        0.0
                    }),
                )
            });
            let focused = response.has_focus() || ui.memory(|m| m.has_focus(ui.id().with(&uid)));
            if delta != Vec2::ZERO && focused && g.layout == Layout::Free {
                if let Some(m) = self
                    .draft
                    .group_mut(&self.group)
                    .unwrap()
                    .members
                    .iter_mut()
                    .find(|m| m.user == uid)
                {
                    if !m.locked {
                        m.x += delta.x;
                        m.y += delta.y;
                    }
                }
            }
        }
    }
    fn characters(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("library")
            .default_size(270.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(16))
            .show(ui, |ui| {
                ui.heading(self.t("Biblioteca", "Library"));
                if ui
                    .button(self.t("Importar imagen o animación", "Import image or animation"))
                    .clicked()
                {
                    self.import_art(String::new(), "idle".into(), ui.ctx());
                }
                if ui
                    .button(self.t("Importar personaje", "Import character"))
                    .clicked()
                {
                    self.package(true, true, ui.ctx());
                }
                ui.add_space(16.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for c in self.draft.characters.clone() {
                        if ui
                            .selectable_label(self.character.as_deref() == Some(&c.id), c.name)
                            .clicked()
                        {
                            self.character = Some(c.id);
                        }
                    }
                });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(24)).show(ui,|ui|{egui::ScrollArea::vertical().show(ui,|ui|{let Some(cid)=self.character.clone()else{ui.heading(self.t("Personajes importados","Imported characters"));ui.label(self.t("Importa el arte y asigna imágenes para cada estado. Podrás reutilizar el personaje en todos tus grupos.","Import artwork and assign images to each state. Reuse the character in all your groups."));return};let es=self.es();let Some(c)=self.draft.characters.iter_mut().find(|c|c.id==cid)else{return};ui.heading(if es{"Editar personaje"}else{"Edit character"});ui.label(if es{"Nombre del personaje"}else{"Character name"});ui.text_edit_singleline(&mut c.name);ui.add_space(16.0);ui.label(RichText::new(if es{"Reacción al hablar"}else{"Speaking reaction"}).strong());ui.add(egui::Slider::new(&mut c.effects.brightness,1.0..=2.0).text(if es{"Iluminación"}else{"Brightness"}));ui.add(egui::Slider::new(&mut c.effects.jump,0.0..=100.0).text(if es{"Salto"}else{"Hop"}));ui.add(egui::Slider::new(&mut c.effects.duration_ms,0..=1000).text(if es{"Transición (ms)"}else{"Transition (ms)"}));ui.add(egui::Slider::new(&mut c.effects.release_ms,0..=1000).text(if es{"Reposo tras hablar (ms)"}else{"Release delay (ms)"}));ui.checkbox(&mut c.effects.glow,if es{"Resplandor"}else{"Glow"});ui.add(egui::Slider::new(&mut c.effects.dim_idle,0.3..=1.0).text(if es{"Brillo en reposo"}else{"Idle brightness"}));ui.add_space(20.0);
            for(state,label)in [("idle",if es{"Reposo"}else{"Idle"}),("speaking",if es{"Hablando"}else{"Speaking"}),("muted",if es{"Silenciado"}else{"Muted"}),("absent",if es{"Ausente"}else{"Absent"})]{self.pose_editor(ui,&cid,state,label,false);}
            ui.add_space(20.0);ui.heading(if es{"Expresiones"}else{"Expressions"});let names=self.draft.characters.iter().find(|c|c.id==cid).unwrap().expressions.keys().cloned().collect::<Vec<_>>();for name in names{self.pose_editor(ui,&cid,&name,&name,true);}ui.horizontal_wrapped(|ui|{ui.label(if es{"Nueva expresión"}else{"New expression"});ui.text_edit_singleline(&mut self.expression_name);if ui.button(if es{"Añadir archivo"}else{"Add file"}).clicked(){if self.expression_name.trim().is_empty()||["show","hide","toggle","reset"].contains(&self.expression_name.as_str()){self.error=Some(if es{"Elige un nombre de expresión distinto de show, hide, toggle o reset"}else{"Choose an expression name other than show, hide, toggle or reset"}.into());}else{self.import_art(cid.clone(),format!("expression:{}",self.expression_name.trim()),ui.ctx());self.expression_name.clear();}}});ui.add_space(20.0);ui.horizontal_wrapped(|ui|{if ui.button(if es{"Exportar personaje"}else{"Export character"}).clicked(){self.package(false,true,ui.ctx());}if ui.button(if es{"Eliminar personaje"}else{"Delete character"}).clicked(){self.delete=Some(("character".into(),cid));}});});});
    }
    fn pose_editor(
        &mut self,
        ui: &mut egui::Ui,
        cid: &str,
        state: &str,
        label: &str,
        expression: bool,
    ) {
        let es = self.es();
        egui::CollapsingHeader::new(label)
            .id_salt((cid, state))
            .default_open(state == "idle")
            .show(ui, |ui| {
                let pose = self
                    .draft
                    .characters
                    .iter()
                    .find(|c| c.id == cid)
                    .and_then(|c| {
                        if expression {
                            c.expressions.get(state)
                        } else {
                            c.states.get(state)
                        }
                    })
                    .cloned();
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(if es { "Elegir archivo" } else { "Choose file" })
                        .clicked()
                    {
                        self.import_art(
                            cid.into(),
                            if expression {
                                format!("expression:{state}")
                            } else {
                                state.into()
                            },
                            ui.ctx(),
                        );
                    }
                    if pose.is_some()
                        && ui
                            .button(if es { "Quitar archivo" } else { "Remove file" })
                            .clicked()
                    {
                        let c = self
                            .draft
                            .characters
                            .iter_mut()
                            .find(|c| c.id == cid)
                            .unwrap();
                        if expression {
                            c.expressions.remove(state);
                        } else {
                            c.states.remove(state);
                        }
                    }
                });
                if let Some(pose) = pose {
                    if let Some(a) = self.draft.assets.get(&pose.asset) {
                        ui.label(&a.name);
                        ui.label(
                            RichText::new(format!(
                                "{} × {} · {}",
                                a.width,
                                a.height,
                                a.extension.to_uppercase()
                            ))
                            .small()
                            .color(MUTED),
                        );
                    }
                    if let Some(texture) = self.texture(&pose.asset, ui.ctx()) {
                        ui.image((texture.id(), texture.size_vec2().normalized() * 120.0));
                    }
                    if let Some(error) = self.thumbs.get(&pose.asset).and_then(|t| t.error.as_ref())
                    {
                        ui.colored_label(ERROR, error);
                    }
                    let c = self
                        .draft
                        .characters
                        .iter_mut()
                        .find(|c| c.id == cid)
                        .unwrap();
                    let Some(p) = (if expression {
                        c.expressions.get_mut(state)
                    } else {
                        c.states.get_mut(state)
                    }) else {
                        return;
                    };
                    ui.label(if es {
                        "Recorte normalizado"
                    } else {
                        "Normalized crop"
                    });
                    for (i, label) in [
                        (0, "X"),
                        (1, "Y"),
                        (2, if es { "Ancho" } else { "Width" }),
                        (3, if es { "Alto" } else { "Height" }),
                    ] {
                        ui.add(
                            egui::Slider::new(
                                &mut p.crop[i],
                                if i < 2 { 0.0..=0.99 } else { 0.01..=1.0 },
                            )
                            .text(label),
                        );
                    }
                    p.crop[2] = p.crop[2].min(1.0 - p.crop[0]);
                    p.crop[3] = p.crop[3].min(1.0 - p.crop[1]);
                    let mut sheet = p.sprite.is_some();
                    if ui
                        .checkbox(
                            &mut sheet,
                            if es {
                                "Hoja de sprites"
                            } else {
                                "Sprite sheet"
                            },
                        )
                        .changed()
                    {
                        p.sprite = sheet.then_some(Sprite {
                            columns: 1,
                            rows: 1,
                            frames: 1,
                            fps: 12.0,
                        });
                    }
                    if let Some(s) = &mut p.sprite {
                        ui.horizontal(|ui| {
                            ui.label(if es { "Columnas" } else { "Columns" });
                            ui.add(egui::DragValue::new(&mut s.columns).range(1..=256));
                            ui.label(if es { "Filas" } else { "Rows" });
                            ui.add(egui::DragValue::new(&mut s.rows).range(1..=256));
                        });
                        ui.add(
                            egui::DragValue::new(&mut s.frames)
                                .range(1..=s.columns * s.rows)
                                .prefix(if es { "Fotogramas: " } else { "Frames: " }),
                        );
                        ui.add(egui::Slider::new(&mut s.fps, 1.0..=60.0).text("FPS"));
                    }
                } else {
                    ui.label(
                        RichText::new(if es {
                            "Usa la imagen de reposo si no asignas otra."
                        } else {
                            "Uses the idle image when no other image is assigned."
                        })
                        .color(MUTED),
                    );
                }
            });
    }
    fn settings(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(24)).show(ui,|ui|{egui::ScrollArea::vertical().show(ui,|ui|{let es=self.es();ui.set_max_width(760.0);ui.heading(if es{"Ajustes"}else{"Settings"});ui.label(if es{"Idioma"}else{"Language"});egui::ComboBox::from_id_salt("language").selected_text(if es{"Español"}else{"English"}).show_ui(ui,|ui|{ui.selectable_value(&mut self.draft.settings.language,"es".into(),"Español");ui.selectable_value(&mut self.draft.settings.language,"en".into(),"English");});ui.checkbox(&mut self.draft.settings.reduced_motion,if es{"Reducir movimiento y pausar animaciones"}else{"Reduce motion and pause animations"});ui.add_space(24.0);ui.heading("Discord");ui.label(if es{"ID público de la aplicación"}else{"Public application ID"});ui.text_edit_singleline(&mut self.draft.settings.discord_app);ui.label(if es{"Registra la app, habilita Public Client y añade esta redirección. El acceso RPC de voz requiere autorización de Discord."}else{"Register the app, enable Public Client and add this redirect. RPC voice access requires Discord authorization."});let redirect=format!("http://127.0.0.1:{}/auth/callback",self.engine.port);ui.horizontal_wrapped(|ui|{ui.monospace(&redirect);if ui.button(if es{"Copiar redirección"}else{"Copy redirect"}).clicked(){ui.ctx().copy_text(redirect);}});ui.hyperlink_to(if es{"Abrir portal de desarrolladores"}else{"Open developer portal"},"https://discord.com/developers/applications");ui.horizontal_wrapped(|ui|{if ui.button(if es{"Guardar ajustes"}else{"Save settings"}).clicked(){self.save_settings();}if ui.button(if es{"Autorizar Discord"}else{"Authorize Discord"}).clicked(){self.save_settings();let _=self.engine.commands.send(Command::Authorize);}if ui.button(if es{"Reconectar"}else{"Reconnect"}).clicked(){let _=self.engine.commands.send(Command::Connect);}if ui.button(if es{"Cerrar sesión"}else{"Sign out"}).clicked(){let _=self.engine.commands.send(Command::Disconnect);}});let runtime=self.engine.runtime.read().unwrap().clone();let mut pinned=self.draft.settings.pinned_channel.is_some();if ui.checkbox(&mut pinned,if es{"Fijar el canal actual"}else{"Pin current channel"}).changed(){self.draft.settings.pinned_channel=if pinned{runtime.channel}else{None};self.save_settings();}
        ui.add_space(24.0);ui.heading(if es{"Salida local"}else{"Local output"});ui.horizontal(|ui|{ui.label(if es{"Puerto (requiere reiniciar)"}else{"Port (restart required)"});ui.add(egui::DragValue::new(&mut self.draft.settings.port).range(1024..=65535));});ui.label(self.engine.store.root.display().to_string());ui.add_space(24.0);ui.heading(if es{"Actualizaciones"}else{"Updates"});ui.label("GitHub owner/repo");ui.text_edit_singleline(&mut self.draft.settings.update_repo);ui.checkbox(&mut self.draft.settings.auto_check,if es{"Buscar actualizaciones al iniciar"}else{"Check for updates on startup"});let update=self.engine.updates.read().unwrap().clone();ui.label(match update.status.as_str(){"checking"=>if es{"Buscando actualizaciones…"}else{"Checking for updates…"},"current"=>if es{"Tienes la última versión"}else{"You're up to date"},"no_releases"=>if es{"Todavía no hay versiones publicadas"}else{"No releases published yet"},"available"=>if es{"Hay una actualización disponible"}else{"An update is available"},"downloading"=>if es{"Descargando y verificando…"}else{"Downloading and verifying…"},"ready"=>if es{"Descarga verificada. Puedes abrir el instalador."}else{"Download verified. You can open the installer."},s=>s});ui.horizontal_wrapped(|ui|{if ui.button(if es{"Buscar actualizaciones"}else{"Check for updates"}).clicked(){self.save_settings();let _=self.engine.commands.send(Command::CheckUpdate);}if update.status=="available"&&ui.button(if es{"Descargar actualización"}else{"Download update"}).clicked(){let _=self.engine.commands.send(Command::InstallUpdate);}if let Some(path)=update.downloaded.as_ref(){if ui.button(if es{"Abrir instalador"}else{"Open installer"}).clicked(){let _=open::that(path);}}if let Some(release)=&update.release{ui.hyperlink_to(if es{"Ver versión en GitHub"}else{"View release on GitHub"},&release.html_url);}});
        ui.add_space(24.0);ui.heading(if es{"Atajos del directo"}else{"Live hotkeys"});ui.label(if es{"Guarda los cambios y registra los atajos con el escritorio."}else{"Save changes and register shortcuts with the desktop."});let people=self.draft.people.values().cloned().collect::<Vec<_>>();for b in &mut self.draft.settings.hotkeys{ui.push_id(&b.id,|ui|{ui.horizontal_wrapped(|ui|{ui.label(if es{"Teclas"}else{"Keys"});ui.text_edit_singleline(&mut b.shortcut);egui::ComboBox::from_id_salt("person").selected_text(self.draft.people.get(&b.user).map(|p|p.name.as_str()).unwrap_or("Guest")).show_ui(ui,|ui|{for p in &people{ui.selectable_value(&mut b.user,p.id.clone(),&p.name);}});ui.label(if es{"Acción / expresión"}else{"Action / expression"});ui.text_edit_singleline(&mut b.action);if ui.button(if es{"Quitar"}else{"Remove"}).clicked(){b.shortcut.clear();}});});}self.draft.settings.hotkeys.retain(|b|!b.shortcut.is_empty());if ui.button(if es{"Añadir atajo"}else{"Add shortcut"}).clicked(){self.draft.settings.hotkeys.push(Binding{id:id(),shortcut:"Ctrl+Alt+1".into(),user:people.first().map(|p|p.id.clone()).unwrap_or_default(),action:"toggle".into()});}ui.label(if es{"Acciones: toggle, show, hide, reset o el nombre de una expresión."}else{"Actions: toggle, show, hide, reset or an expression name."});if ui.button(if es{"Registrar atajos"}else{"Register hotkeys"}).clicked(){self.save_settings();let _=self.engine.commands.send(Command::RegisterHotkeys);}
        ui.add_space(24.0);if ui.button(if es{"Exportar diagnóstico"}else{"Export diagnostics"}).clicked(){self.diagnostics(ui.ctx());}ui.label(format!("Charlita {} · {} / {}",env!("CARGO_PKG_VERSION"),std::env::consts::OS,std::env::consts::ARCH));ui.hyperlink_to(if es{"Código fuente y ayuda"}else{"Source code and help"},"https://github.com/48hoursnonstop/charlita");});});
    }
    fn save_settings(&mut self) {
        let mut live = self.engine.live.read().unwrap().clone();
        if live.settings.discord_app != self.draft.settings.discord_app {
            *self.engine.auth.lock().unwrap() = Default::default();
        }
        live.settings = self.draft.settings.clone();
        let r = self
            .engine
            .store
            .save("live", &live)
            .and_then(|_| self.engine.store.save("draft", &self.draft));
        if let Err(e) = r {
            self.error = Some(e.to_string());
            return;
        }
        *self.engine.live.write().unwrap() = live;
        self.engine.notify();
        self.message = Some(self.t("Ajustes guardados", "Settings saved").into());
    }
    fn diagnostics(&mut self, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let runtime = self.engine.runtime.read().unwrap().clone();
        let info = serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"connection":runtime.connection,"people":runtime.users.len(),"profiles":self.draft.profiles.len(),"characters":self.draft.characters.len(),"assets":self.draft.assets.len(),"ffmpeg_available":std::process::Command::new(assets::media_tool("ffmpeg")).arg("-version").output().is_ok()});
        self.busy += 1;
        std::thread::spawn(move || {
            let r = rfd::FileDialog::new()
                .set_file_name("charlita-diagnostics.json")
                .save_file()
                .map(|p| {
                    std::fs::write(p, serde_json::to_vec_pretty(&info).unwrap())
                        .map(|_| "Diagnostics exported / Diagnóstico exportado".into())
                        .map_err(anyhow::Error::from)
                })
                .unwrap_or_else(|| Err(anyhow::anyhow!("cancelled")));
            let _ = tx.send(Work::Message(r));
            ctx.request_repaint();
        });
    }
    fn confirm_delete(&mut self, ctx: &egui::Context) {
        let Some((kind, id)) = self.delete.clone() else {
            return;
        };
        let es = self.es();
        egui::Modal::new(egui::Id::new("confirm_delete")).show(ctx, |ui| {
            ui.heading(if es {
                "Confirmar eliminación"
            } else {
                "Confirm deletion"
            });
            ui.label(if es {
                "Se eliminará del borrador. Puedes deshacerlo antes de aplicar."
            } else {
                "This removes it from the draft. You can undo before applying."
            });
            ui.horizontal(|ui| {
                if ui.button(if es { "Cancelar" } else { "Cancel" }).clicked() {
                    self.delete = None;
                }
                if ui
                    .add(
                        egui::Button::new(if es { "Eliminar" } else { "Delete" })
                            .fill(Color32::from_rgb(91, 40, 47)),
                    )
                    .clicked()
                {
                    self.undo.push(self.draft.clone());
                    match kind.as_str() {
                        "profile" => {
                            self.draft.profiles.retain(|p| p.id != id);
                        }
                        "group" => {
                            self.draft.profiles[self.profile]
                                .groups
                                .retain(|g| g.id != id);
                        }
                        "character" => {
                            self.draft.characters.retain(|c| c.id != id);
                            for p in self.draft.people.values_mut() {
                                if p.character.as_deref() == Some(&id) {
                                    p.character = None
                                }
                            }
                            for p in &mut self.draft.profiles {
                                for g in &mut p.groups {
                                    for m in &mut g.members {
                                        if m.character.as_deref() == Some(&id) {
                                            m.character = None
                                        }
                                    }
                                }
                            }
                            self.character = None;
                        }
                        _ => {}
                    }
                    self.delete = None;
                    self.dirty = true;
                    self.normalize_selection();
                }
            });
        });
    }
}
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.process(ctx);
        if let Some(message) = self.engine.notice.lock().unwrap().take() {
            self.error = Some(message);
        }
        while let Ok(cmd) = self.native_rx.try_recv() {
            match cmd {
                Command::Show => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                Command::Quit => {
                    self.quit = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Command::RegisterHotkeys => {
                    if let Err(error) = self.native.register() {
                        self.error = Some(error.to_string());
                    }
                }
                _ => {}
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.quit && self.tray_available {
            for animation in self.animated.values() {
                animation.player.play(false);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if self.dirty && self.last_edit.elapsed() > Duration::from_millis(600) {
            if let Err(e) = self.engine.store.save("draft", &self.draft) {
                self.error = Some(e.to_string());
            }
            self.last_edit = Instant::now() + Duration::from_secs(3600);
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        // Players used by this frame are resumed by animated_texture. Other
        // views and minimized windows retain a still frame without decoding.
        let frame_started = Instant::now();
        let before = self.draft.clone();
        let ctx = ui.ctx().clone();
        self.header(ui);
        self.footer(ui);
        match self.view {
            0 => self.groups(ui),
            1 => self.characters(ui),
            _ => self.settings(ui),
        }
        self.confirm_delete(&ctx);
        for animation in self.animated.values() {
            if animation.last < frame_started {
                animation.player.play(false);
            }
        }
        if self.draft != before {
            self.undo.push(before);
            if self.undo.len() > 40 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.dirty = true;
            self.last_edit = Instant::now();
            ctx.request_repaint_after(Duration::from_millis(650));
        }
        if self.busy > 0 {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    fn save(&mut self, _: &mut dyn eframe::Storage) {
        if let Err(e) = self.engine.store.save("draft", &self.draft) {
            tracing::error!(error=%e,"Could not save draft");
        }
    }
}
fn layout_name(l: Layout, es: bool) -> &'static str {
    match (l, es) {
        (Layout::Row, true) => "Fila horizontal",
        (Layout::Row, false) => "Horizontal row",
        (Layout::Column, true) => "Columna vertical",
        (Layout::Column, false) => "Vertical column",
        (Layout::Grid, true) => "Cuadrícula",
        (Layout::Grid, false) => "Grid",
        (Layout::Free, true) => "Libre",
        (Layout::Free, false) => "Free",
    }
}
fn character_picker(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    value: &mut Option<String>,
    characters: &[Character],
    es: bool,
) {
    ui.label(label);
    egui::ComboBox::from_id_salt(id)
        .selected_text(
            value
                .as_ref()
                .and_then(|id| characters.iter().find(|c| &c.id == id))
                .map(|c| c.name.as_str())
                .unwrap_or(if es { "Automático" } else { "Automatic" }),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, if es { "Automático" } else { "Automatic" });
            for c in characters {
                ui.selectable_value(value, Some(c.id.clone()), &c.name);
            }
        });
}
