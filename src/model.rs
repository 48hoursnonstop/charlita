use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub fn id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Document {
    pub version: u32,
    pub profiles: Vec<Profile>,
    pub people: BTreeMap<String, Person>,
    pub characters: Vec<Character>,
    pub assets: BTreeMap<String, Asset>,
    pub settings: Settings,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: vec![Profile::new("Principal")],
            people: BTreeMap::new(),
            characters: vec![],
            assets: BTreeMap::new(),
            settings: Settings::default(),
        }
    }
}
impl Document {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1,
            "Unsupported document version / Versión de documento no compatible"
        );
        ensure!(
            !self.profiles.is_empty(),
            "Keep at least one profile / Conserva al menos un perfil"
        );
        let mut ids = BTreeSet::new();
        for asset in self.assets.values() {
            Uuid::parse_str(&asset.id)?;
            ensure!(
                matches!(
                    asset.extension.as_str(),
                    "png" | "jpg" | "jpeg" | "webp" | "gif" | "webm"
                ),
                "Unsupported asset format"
            );
            ensure!(
                asset.width > 0 && asset.height > 0,
                "Invalid image dimensions"
            );
            ensure!(
                self.assets.get(&asset.id) == Some(asset),
                "Asset identity mismatch"
            );
        }
        for c in &self.characters {
            ensure!(ids.insert(c.id.clone()), "Duplicate character");
            Uuid::parse_str(&c.id)?;
            ensure!(
                c.effects.brightness.is_finite() && (1.0..=3.0).contains(&c.effects.brightness),
                "Invalid brightness"
            );
            ensure!(
                c.effects.jump.is_finite() && (0.0..=200.0).contains(&c.effects.jump),
                "Invalid jump"
            );
            for pose in c.states.values().chain(c.expressions.values()) {
                ensure!(
                    self.assets.contains_key(&pose.asset),
                    "Missing asset / Falta un archivo del personaje"
                );
                ensure!(
                    pose.crop
                        .iter()
                        .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
                        && pose.crop[0] + pose.crop[2] <= 1.001
                        && pose.crop[1] + pose.crop[3] <= 1.001
                        && pose.crop[2] > 0.0
                        && pose.crop[3] > 0.0,
                    "Invalid crop"
                );
                if let Some(s) = &pose.sprite {
                    ensure!(
                        s.columns > 0
                            && s.rows > 0
                            && s.frames > 0
                            && s.frames <= s.columns.saturating_mul(s.rows)
                            && s.fps.is_finite()
                            && (1.0..=60.0).contains(&s.fps),
                        "Invalid sprite sheet"
                    );
                }
            }
        }
        let characters: BTreeSet<_> = self.characters.iter().map(|c| c.id.as_str()).collect();
        for person in self.people.values() {
            if let Some(c) = &person.character {
                ensure!(characters.contains(c.as_str()), "Missing character");
            }
        }
        for p in &self.profiles {
            Uuid::parse_str(&p.id)?;
            ensure!(!p.groups.is_empty(), "Keep at least one group");
            for g in &p.groups {
                Uuid::parse_str(&g.id)?;
                ensure!(ids.insert(g.id.clone()), "Duplicate group");
                ensure!(
                    (64..=8192).contains(&g.width) && (64..=8192).contains(&g.height),
                    "Output size must be 64–8192 px"
                );
                ensure!(
                    g.gap.is_finite() && (0.0..=500.0).contains(&g.gap),
                    "Invalid gap"
                );
                let mut members = BTreeSet::new();
                for m in &g.members {
                    ensure!(
                        self.people.contains_key(&m.user) && members.insert(m.user.clone()),
                        "Invalid or duplicate guest"
                    );
                    if let Some(c) = &m.character {
                        ensure!(
                            characters.contains(c.as_str()),
                            "Missing character override"
                        );
                    }
                    ensure!(
                        [m.x, m.y, m.size].iter().all(|x| x.is_finite())
                            && (16.0..=4096.0).contains(&m.size),
                        "Invalid guest geometry"
                    );
                }
            }
        }
        ensure!(
            (1024..=65535).contains(&self.settings.port),
            "Choose a port between 1024 and 65535"
        );
        Ok(())
    }
    pub fn group(&self, id: &str) -> Option<&Group> {
        self.profiles
            .iter()
            .flat_map(|p| &p.groups)
            .find(|g| g.id == id)
    }
    pub fn group_mut(&mut self, id: &str) -> Option<&mut Group> {
        self.profiles
            .iter_mut()
            .flat_map(|p| &mut p.groups)
            .find(|g| g.id == id)
    }
    pub fn character_for(&self, m: &Member) -> Option<&Character> {
        let id = m
            .character
            .as_ref()
            .or_else(|| self.people.get(&m.user).and_then(|p| p.character.as_ref()))?;
        self.characters.iter().find(|c| &c.id == id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub groups: Vec<Group>,
}
impl Profile {
    pub fn new(name: &str) -> Self {
        Self {
            id: id(),
            name: name.into(),
            groups: vec![Group::new("Invitados")],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub layout: Layout,
    pub width: u32,
    pub height: u32,
    pub gap: f32,
    pub columns: u32,
    pub preserve_spaces: bool,
    pub labels: bool,
    pub members: Vec<Member>,
}
impl Default for Group {
    fn default() -> Self {
        Self::new("Invitados")
    }
}
impl Group {
    pub fn new(name: &str) -> Self {
        Self {
            id: id(),
            name: name.into(),
            layout: Layout::Row,
            width: 1280,
            height: 400,
            gap: 24.0,
            columns: 3,
            preserve_spaces: false,
            labels: true,
            members: vec![],
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    Row,
    Column,
    Grid,
    Free,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub character: Option<String>,
    pub avatar: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Member {
    pub user: String,
    pub enabled: bool,
    pub character: Option<String>,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub mirror: bool,
    pub locked: bool,
}
impl Default for Member {
    fn default() -> Self {
        Self {
            user: String::new(),
            enabled: true,
            character: None,
            x: 100.0,
            y: 60.0,
            size: 240.0,
            mirror: false,
            locked: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub states: BTreeMap<String, Pose>,
    pub expressions: BTreeMap<String, Pose>,
    pub effects: Effects,
}
impl Character {
    pub fn new(name: &str) -> Self {
        Self {
            id: id(),
            name: name.into(),
            states: BTreeMap::new(),
            expressions: BTreeMap::new(),
            effects: Effects::default(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pose {
    pub asset: String,
    pub crop: [f32; 4],
    pub sprite: Option<Sprite>,
}
impl Pose {
    pub fn new(asset: String) -> Self {
        Self {
            asset,
            crop: [0.0, 0.0, 1.0, 1.0],
            sprite: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Sprite {
    pub columns: u32,
    pub rows: u32,
    pub frames: u32,
    pub fps: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Effects {
    pub brightness: f32,
    pub jump: f32,
    pub duration_ms: u32,
    pub release_ms: u32,
    pub glow: bool,
    pub dim_idle: f32,
}
impl Default for Effects {
    fn default() -> Self {
        Self {
            brightness: 1.12,
            jump: 10.0,
            duration_ms: 180,
            release_ms: 160,
            glow: false,
            dim_idle: 1.0,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
}
impl Asset {
    pub fn filename(&self) -> String {
        format!("{}.{}", self.id, self.extension)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub port: u16,
    pub discord_app: String,
    pub update_repo: String,
    pub auto_check: bool,
    pub reduced_motion: bool,
    pub pinned_channel: Option<String>,
    pub hotkeys: Vec<Binding>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: sys_locale::get_locale()
                .filter(|s| s.starts_with("es"))
                .map(|_| "es".into())
                .unwrap_or("en".into()),
            port: 38465,
            discord_app: String::new(),
            update_repo: "48hoursnonstop/charlita".into(),
            auto_check: true,
            reduced_motion: false,
            pinned_channel: None,
            hotkeys: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Binding {
    pub id: String,
    pub shortcut: String,
    pub user: String,
    pub action: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Presence {
    pub speaking: bool,
    pub muted: bool,
    pub present: bool,
    pub expression: Option<String>,
    pub hidden: Option<bool>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Runtime {
    pub connection: String,
    pub channel: Option<String>,
    pub channel_name: String,
    pub users: BTreeMap<String, Presence>,
    pub discovered: BTreeMap<String, Person>,
    pub test: BTreeMap<String, Presence>,
}
