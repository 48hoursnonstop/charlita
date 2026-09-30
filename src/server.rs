use crate::{engine::Engine, output};
use axum::{
    Router,
    body::Body,
    extract::{
        Path, State,
        ws::{Message, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub async fn serve(engine: Engine, listener: tokio::net::TcpListener) -> anyhow::Result<()> {
    let app=Router::new().route("/",get(||async{Html("<!doctype html><title>Charlita</title><p>Charlita is running. Copy an overlay URL from the desktop app.</p>")}))
        .route("/o/{key}/group/{id}",get(page_group)).route("/o/{key}/person/{id}",get(page_person))
        .route("/o/{key}/state/{kind}/{id}",get(state)).route("/o/{key}/ws/{kind}/{id}",get(ws)).route("/o/{key}/asset/{id}",get(asset))
        .route("/o/{key}/asset/{id}/still",get(still))
        .route("/static/overlay.js",get(||async{([(header::CONTENT_TYPE,"text/javascript; charset=utf-8"),(header::CACHE_CONTROL,"no-cache")],include_str!("../overlay/overlay.js"))}))
        .route("/static/overlay.css",get(||async{([(header::CONTENT_TYPE,"text/css; charset=utf-8")],include_str!("../overlay/overlay.css"))}))
        .route("/auth/callback",get(crate::discord::auth_callback))
        .route("/control/{key}/show",post(show)).with_state(engine);
    axum::serve(listener, app).await?;
    Ok(())
}
async fn show(State(e): State<Engine>, Path(key): Path<String>) -> StatusCode {
    if !allowed(&e, &key) {
        return StatusCode::NOT_FOUND;
    }
    let _ = e.commands.send(crate::engine::Command::Show);
    StatusCode::NO_CONTENT
}
fn allowed(e: &Engine, key: &str) -> bool {
    e.key == key
}
async fn page_group(State(e): State<Engine>, Path((key, id)): Path<(String, String)>) -> Response {
    page(&e, &key, "group", &id)
}
async fn page_person(State(e): State<Engine>, Path((key, id)): Path<(String, String)>) -> Response {
    page(&e, &key, "person", &id)
}
fn page(e: &Engine, key: &str, kind: &str, id: &str) -> Response {
    if !allowed(e, key) || snapshot(e, kind, id).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    Html(include_str!("../overlay/index.html")).into_response()
}
fn snapshot(e: &Engine, kind: &str, id: &str) -> Option<output::Output> {
    let d = e.live.read().unwrap();
    let r = e.runtime.read().unwrap();
    if kind == "group" {
        Some(output::group(&d, &r, d.group(id)?, &e.key, false))
    } else if kind == "person" {
        output::individual(&d, &r, id, &e.key)
    } else {
        None
    }
}
async fn state(
    State(e): State<Engine>,
    Path((key, kind, id)): Path<(String, String, String)>,
) -> Response {
    if !allowed(&e, &key) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match snapshot(&e, &kind, &id) {
        Some(v) => axum::Json(v).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn ws(
    State(e): State<Engine>,
    Path((key, kind, id)): Path<(String, String, String)>,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !allowed(&e, &key) || snapshot(&e, &kind, &id).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    upgrade.on_upgrade(move|socket|async move{
        let(mut tx,mut rx)=socket.split();let mut events=e.events.subscribe();let mut heartbeat=tokio::time::interval(Duration::from_secs(20));
        loop{
            if let Some(v)=snapshot(&e,&kind,&id){let text=serde_json::to_string(&v).unwrap();if tx.send(Message::Text(text.into())).await.is_err(){break}}else{let _=tx.send(Message::Close(None)).await;break}
            loop{tokio::select!{
                event=events.recv()=>{match event{Ok(())|Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>break,Err(_)=>return}},
                event=rx.next()=>{match event{None|Some(Err(_))|Some(Ok(Message::Close(_)))=>return,Some(Ok(Message::Ping(bytes)))=>{if tx.send(Message::Pong(bytes)).await.is_err(){return}},_=>{}}},
                _=heartbeat.tick()=>{if tx.send(Message::Ping(vec![].into())).await.is_err(){return}}
            }}
        }
    })
}
async fn asset(
    State(e): State<Engine>,
    Path((key, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !allowed(&e, &key) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let asset = e.live.read().unwrap().assets.get(&id).cloned();
    let Some(a) = asset else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(mut file) = tokio::fs::File::open(e.store.asset_path(&a)).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(meta) = file.metadata().await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, a.mime)
        .header(
            header::CACHE_CONTROL,
            "private, max-age=31536000, immutable",
        )
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::ACCEPT_RANGES, "bytes");
    if let Some(value) = headers.get(header::RANGE) {
        let range = value
            .to_str()
            .ok()
            .and_then(|value| byte_range(value, meta.len()));
        let Some((start, end)) = range else {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [(header::CONTENT_RANGE, format!("bytes */{}", meta.len()))],
            )
                .into_response();
        };
        if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        let length = end - start + 1;
        response = response
            .status(StatusCode::PARTIAL_CONTENT)
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{}", meta.len()),
            )
            .header(header::CONTENT_LENGTH, length);
        return response
            .body(Body::from_stream(tokio_util::io::ReaderStream::new(
                file.take(length),
            )))
            .unwrap();
    }
    response
        .header(header::CONTENT_LENGTH, meta.len())
        .body(Body::from_stream(tokio_util::io::ReaderStream::new(file)))
        .unwrap()
}

fn byte_range(value: &str, length: u64) -> Option<(u64, u64)> {
    if length == 0 {
        return None;
    }
    let (start, end) = value.strip_prefix("bytes=")?.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().ok()?;
        return (suffix > 0).then_some((length.saturating_sub(suffix), length - 1));
    }
    let start = start.parse::<u64>().ok()?;
    let end = if end.is_empty() {
        length - 1
    } else {
        end.parse::<u64>().ok()?.min(length - 1)
    };
    (start <= end && start < length).then_some((start, end))
}
async fn still(State(e): State<Engine>, Path((key, id)): Path<(String, String)>) -> Response {
    if !allowed(&e, &key) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(asset) = e.live.read().unwrap().assets.get(&id).cloned() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let root = e.store.root.join("thumbnails");
    let path = root.join(format!("{id}.png"));
    if !path.is_file() {
        let store = e.store.clone();
        let dest = path.clone();
        let result = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            std::fs::create_dir_all(&root)?;
            let image = crate::assets::thumbnail(&store, &asset)?;
            let temporary = tempfile::NamedTempFile::new_in(root)?;
            image.save_with_format(temporary.path(), image::ImageFormat::Png)?;
            temporary.persist(dest)?;
            Ok(())
        })
        .await;
        if !matches!(result, Ok(Ok(()))) {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    match tokio::fs::read(path).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png"),
                (
                    header::CACHE_CONTROL,
                    "private, max-age=31536000, immutable",
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{Member, Person, Presence},
        store::Store,
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn published_output_live_controls_and_media_work_over_http_and_websocket() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path().join("data")).unwrap());
        let (e, _) = Engine::new(store.clone()).unwrap();
        let mut doc = e.live.read().unwrap().clone();
        doc.people.insert(
            "1".into(),
            Person {
                id: "1".into(),
                name: "Invitada".into(),
                character: None,
                avatar: None,
            },
        );
        doc.profiles[0].groups[0].members.push(Member {
            user: "1".into(),
            ..Default::default()
        });
        let group = doc.profiles[0].groups[0].id.clone();
        let path = dir.path().join("image.png");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 128]))
            .save(&path)
            .unwrap();
        let asset = crate::assets::import(&store, &path).unwrap();
        doc.assets.insert(asset.id.clone(), asset.clone());
        e.apply(&doc).unwrap();
        e.runtime.write().unwrap().users.insert(
            "1".into(),
            Presence {
                present: true,
                ..Default::default()
            },
        );
        e.connection("connected");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(serve(e.clone(), listener));
        let base = format!("http://{address}/o/{}", e.key);
        let http = reqwest::Client::new();
        assert_eq!(
            http.get(format!("http://{address}/o/invalid/group/{group}"))
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
        let (mut socket, _) = tokio_tungstenite::connect_async(format!(
            "ws://{address}/o/{}/ws/group/{group}",
            e.key
        ))
        .await
        .unwrap();
        async fn next_state(
            socket: &mut tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        ) -> serde_json::Value {
            tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    let frame = socket.next().await.unwrap().unwrap();
                    if frame.is_text() {
                        return serde_json::from_str(frame.to_text().unwrap()).unwrap();
                    }
                }
            })
            .await
            .unwrap()
        }
        assert_eq!(
            next_state(&mut socket).await["guests"][0]["name"],
            "Invitada"
        );
        doc.people.get_mut("1").unwrap().name = "Cambio sin aplicar".into();
        doc.profiles[0].groups[0].members[0].size = 300.0;
        store.save("draft", &doc).unwrap();
        let state: serde_json::Value = http
            .get(format!("{base}/state/group/{group}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(state["guests"][0]["name"], "Invitada");
        assert_eq!(state["width"], 280);
        e.test("1", "speaking");
        let state: serde_json::Value = http
            .get(format!("{base}/state/group/{group}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(state["guests"][0]["state"], "idle");
        e.apply(&doc).unwrap();
        let applied = next_state(&mut socket).await;
        assert_eq!(applied["guests"][0]["name"], "Cambio sin aplicar");
        assert_eq!(applied["width"], 340);
        e.live_action("1", "hide");
        assert_eq!(
            next_state(&mut socket).await["guests"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        e.live_action("1", "show");
        assert_eq!(
            next_state(&mut socket).await["guests"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        e.runtime
            .write()
            .unwrap()
            .users
            .get_mut("1")
            .unwrap()
            .speaking = true;
        e.notify();
        assert_eq!(
            next_state(&mut socket).await["guests"][0]["state"],
            "speaking"
        );
        e.connection("reconnecting");
        assert_eq!(next_state(&mut socket).await["guests"][0]["state"], "idle");
        let response = http
            .get(format!("{base}/asset/{}", asset.id))
            .header("Range", "bytes=0-9")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.bytes().await.unwrap().len(), 10);
        assert_eq!(
            http.get(format!("{base}/asset/{}", asset.id))
                .header("Range", "bytes=999999-")
                .send()
                .await
                .unwrap()
                .status(),
            416
        );
        let bytes = http
            .get(format!("{base}/asset/{}/still", asset.id))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(image::load_from_memory(&bytes).unwrap().width(), 4);
        socket.close(None).await.unwrap();
        server.abort();
    }
}
