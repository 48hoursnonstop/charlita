use crate::{
    layout,
    model::{Document, Effects, Group, Member, Pose, Presence, Runtime},
};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Output {
    pub width: u32,
    pub height: u32,
    pub labels: bool,
    pub reduced_motion: bool,
    pub connected: bool,
    pub guests: Vec<Guest>,
}
#[derive(Clone, Serialize)]
pub struct Guest {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub mirror: bool,
    pub visible: bool,
    pub state: String,
    pub effects: Effects,
    pub pose: Option<Pose>,
    pub media: Option<Media>,
    pub idle_pose: Option<Pose>,
    pub idle_media: Option<Media>,
}
#[derive(Clone, Serialize)]
pub struct Media {
    pub url: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
}
pub fn group(doc: &Document, runtime: &Runtime, g: &Group, key: &str, preview: bool) -> Output {
    let visible: Vec<_> = g
        .members
        .iter()
        .map(|m| {
            m.enabled
                && !runtime
                    .users
                    .get(&m.user)
                    .and_then(|p| p.hidden)
                    .unwrap_or(false)
        })
        .collect();
    let positions = layout::positions(g, &visible);
    let characters: std::collections::BTreeMap<_, _> =
        doc.characters.iter().map(|c| (c.id.as_str(), c)).collect();
    let guests = g
        .members
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let pos = positions[i]?;
            let person = doc.people.get(&m.user)?;
            let p = if preview {
                runtime
                    .test
                    .get(&m.user)
                    .or_else(|| runtime.users.get(&m.user))
            } else {
                runtime.users.get(&m.user)
            };
            let empty = Presence::default();
            let p = p.unwrap_or(&empty);
            let state = if !p.present {
                "absent"
            } else if p.muted {
                "muted"
            } else if p.speaking && runtime.connection == "connected" || preview && p.speaking {
                "speaking"
            } else {
                "idle"
            };
            let c = m
                .character
                .as_ref()
                .or(person.character.as_ref())
                .and_then(|id| characters.get(id.as_str()).copied());
            let effects = c.map(|c| c.effects.clone()).unwrap_or_default();
            let pose = c
                .and_then(|c| {
                    p.expression
                        .as_ref()
                        .and_then(|x| c.expressions.get(x))
                        .or_else(|| c.states.get(state))
                        .or_else(|| c.states.get("idle"))
                })
                .cloned();
            let media = pose
                .as_ref()
                .and_then(|p| doc.assets.get(&p.asset))
                .map(|a| Media {
                    url: format!("/o/{key}/asset/{}", a.id),
                    kind: a.extension.clone(),
                    width: a.width,
                    height: a.height,
                })
                .or_else(|| {
                    person.avatar.as_ref().map(|url| Media {
                        url: url.clone(),
                        kind: "png".into(),
                        width: 256,
                        height: 256,
                    })
                });
            let idle_pose = c
                .and_then(|c| {
                    p.expression
                        .as_ref()
                        .and_then(|x| c.expressions.get(x))
                        .or_else(|| c.states.get("idle"))
                })
                .cloned();
            let idle_media = idle_pose
                .as_ref()
                .and_then(|p| doc.assets.get(&p.asset))
                .map(|a| Media {
                    url: format!("/o/{key}/asset/{}", a.id),
                    kind: a.extension.clone(),
                    width: a.width,
                    height: a.height,
                })
                .or_else(|| {
                    person.avatar.as_ref().map(|url| Media {
                        url: url.clone(),
                        kind: "png".into(),
                        width: 256,
                        height: 256,
                    })
                });
            Some(Guest {
                id: m.user.clone(),
                name: person.name.clone(),
                x: pos.x,
                y: pos.y,
                size: pos.w,
                mirror: m.mirror,
                visible: visible[i],
                state: state.into(),
                effects,
                pose,
                media,
                idle_pose,
                idle_media,
            })
        })
        .collect();
    Output {
        width: g.width,
        height: g.height,
        labels: g.labels,
        reduced_motion: doc.settings.reduced_motion,
        connected: runtime.connection == "connected",
        guests,
    }
}
pub fn individual(doc: &Document, r: &Runtime, user: &str, key: &str) -> Option<Output> {
    doc.people.get(user)?;
    let g = Group {
        id: user.into(),
        width: 400,
        height: 440,
        labels: true,
        members: vec![Member {
            user: user.into(),
            x: 50.0,
            y: 40.0,
            size: 300.0,
            ..Default::default()
        }],
        layout: crate::model::Layout::Free,
        ..Default::default()
    };
    Some(group(doc, r, &g, key, false))
}
