use crate::{
    engine::Engine,
    model::{Person, Presence},
};
use anyhow::{Context, Result, bail, ensure};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    pin::Pin,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Default)]
pub struct AuthState {
    pending: Option<Pending>,
    token: Option<Token>,
}
struct Pending {
    state: String,
    verifier: String,
    started: Instant,
}
#[derive(Clone, Serialize, Deserialize)]
struct Token {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}
#[derive(Deserialize)]
pub struct Callback {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn entry(app: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(
        "io.github.48hoursnonstop.charlita",
        &format!("discord-{app}"),
    )?)
}
fn credentials(e: &Engine) -> Option<Token> {
    let mut auth = e.auth.lock().unwrap();
    if auth.token.is_none() {
        let app = e.live.read().unwrap().settings.discord_app.clone();
        auth.token = entry(&app)
            .ok()
            .and_then(|k| k.get_password().ok())
            .and_then(|s| serde_json::from_str(&s).ok());
    }
    auth.token.clone()
}
fn save_token(e: &Engine, token: Token) {
    let app = e.live.read().unwrap().settings.discord_app.clone();
    if let Ok(k) = entry(&app)
        && k.set_password(&serde_json::to_string(&token).unwrap())
            .is_err()
    {
        tracing::warn!("Credentials remain in memory: the OS credential store is unavailable");
    }
    e.auth.lock().unwrap().token = Some(token);
}
pub fn logout(e: &Engine) {
    let app = e.live.read().unwrap().settings.discord_app.clone();
    if let Ok(k) = entry(&app) {
        let _ = k.delete_credential();
    }
    *e.auth.lock().unwrap() = AuthState::default();
    e.connection("signed_out");
}
pub async fn authorize(e: Engine) -> Result<()> {
    let settings = e.live.read().unwrap().settings.clone();
    ensure!(
        !settings.discord_app.is_empty()
            && settings.discord_app.chars().all(|c| c.is_ascii_digit()),
        "Set the application's public ID in Settings / Introduce el ID público en Ajustes"
    );
    let verifier = format!(
        "{}{}",
        crate::model::id().replace('-', ""),
        crate::model::id().replace('-', "")
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = crate::model::id();
    let redirect = format!("http://127.0.0.1:{}/auth/callback", e.port);
    let mut url = reqwest::Url::parse("https://discord.com/oauth2/authorize")?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", settings.discord_app.as_str()),
        ("response_type", "code"),
        ("redirect_uri", &redirect),
        ("scope", "rpc rpc.voice.read identify"),
        ("state", &state),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("prompt", "consent"),
    ]);
    e.auth.lock().unwrap().pending = Some(Pending {
        state,
        verifier,
        started: Instant::now(),
    });
    open::that(url.as_str())?;
    e.connection("authorizing");
    Ok(())
}
pub async fn auth_callback(
    State(e): State<Engine>,
    Query(q): Query<Callback>,
) -> (StatusCode, Html<String>) {
    let result = finish_auth(&e, q).await;
    let (status, msg) = match result {
        Ok(()) => (
            StatusCode::OK,
            "Connected. Return to Charlita / Conectado. Vuelve a Charlita".to_owned(),
        ),
        Err(err) => {
            e.connection("authorization_failed");
            tracing::warn!("Discord authorization failed");
            (
                StatusCode::BAD_REQUEST,
                format!(
                    "Authorization failed. Check application permissions and try again in Charlita / Falló la autorización. Revisa los permisos y vuelve a intentarlo. ({})",
                    escape(&err.to_string())
                ),
            )
        }
    };
    (
        status,
        Html(format!(
            "<!doctype html><meta charset=utf-8><title>Charlita</title><style>body{{font:18px system-ui;background:#15181d;color:#edf0f5;max-width:650px;margin:12vh auto;padding:24px}}</style><h1>Charlita</h1><p>{msg}</p>"
        )),
    )
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
async fn finish_auth(e: &Engine, q: Callback) -> Result<()> {
    let pending = {
        let mut auth = e.auth.lock().unwrap();
        let p = auth
            .pending
            .as_ref()
            .context("No authorization in progress")?;
        ensure!(
            Some(p.state.as_str()) == q.state.as_deref()
                && p.started.elapsed() < Duration::from_secs(600),
            "Authorization expired or state mismatch"
        );
        auth.pending.take().unwrap()
    };
    ensure!(q.error.is_none(), "Authorization was declined");
    let code = q.code.context("No authorization code")?;
    let s = e.live.read().unwrap().settings.clone();
    let redirect = format!("http://127.0.0.1:{}/auth/callback", e.port);
    let response = http()?
        .post("https://discord.com/api/v10/oauth2/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", &s.discord_app),
            ("code", &code),
            ("redirect_uri", &redirect),
            ("code_verifier", &pending.verifier),
        ])
        .send()
        .await?;
    ensure!(
        response.status().is_success(),
        "Discord rejected authorization. Enable Public Client and the required RPC voice permissions"
    );
    let v: Value = response.json().await?;
    save_token(e, token_from(&v)?);
    e.connection("connecting");
    let _ = e.commands.send(crate::engine::Command::Connect);
    Ok(())
}
fn token_from(v: &Value) -> Result<Token> {
    Ok(Token {
        access_token: v["access_token"]
            .as_str()
            .context("No access token")?
            .into(),
        refresh_token: v["refresh_token"].as_str().unwrap_or("").into(),
        expires_at: now() + v["expires_in"].as_u64().unwrap_or(3600),
    })
}
fn http() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("Charlita/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(15))
        .build()?)
}
async fn access(e: &Engine) -> Result<String> {
    let mut token =
        credentials(e).context("Authorize Discord in Settings / Autoriza Discord en Ajustes")?;
    if token.expires_at < now() + 60 {
        ensure!(!token.refresh_token.is_empty(), "Authorize Discord again");
        let s = e.live.read().unwrap().settings.clone();
        let r = http()?
            .post("https://discord.com/api/v10/oauth2/token")
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", &s.discord_app),
                ("refresh_token", &token.refresh_token),
            ])
            .send()
            .await?;
        if !r.status().is_success() {
            logout(e);
            bail!("Authorize Discord again")
        }
        token = token_from(&r.json::<Value>().await?)?;
        save_token(e, token.clone());
    }
    Ok(token.access_token)
}

trait Io: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> Io for T {}
type Stream = Pin<Box<dyn Io>>;
async fn connect() -> Result<Stream> {
    #[cfg(unix)]
    {
        let mut dirs = vec![];
        for key in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
            if let Ok(p) = std::env::var(key) {
                dirs.push(std::path::PathBuf::from(p));
            }
        }
        dirs.push("/tmp".into());
        let initial = dirs.clone();
        for d in initial {
            dirs.push(d.join("app/com.discordapp.Discord"));
            dirs.push(d.join("snap.discord"));
        }
        for dir in dirs {
            for n in 0..10 {
                if let Ok(s) =
                    tokio::net::UnixStream::connect(dir.join(format!("discord-ipc-{n}"))).await
                {
                    return Ok(Box::pin(s));
                }
            }
        }
    }
    #[cfg(windows)]
    {
        for n in 0..10 {
            if let Ok(s) = tokio::net::windows::named_pipe::ClientOptions::new()
                .open(format!(r"\\?\pipe\discord-ipc-{n}"))
            {
                return Ok(Box::pin(s));
            }
        }
    }
    bail!("Open the official Discord desktop app / Abre Discord de escritorio")
}
pub async fn write_frame<W: AsyncWrite + Unpin>(s: &mut W, opcode: u32, v: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(v)?;
    s.write_u32_le(opcode).await?;
    s.write_u32_le(bytes.len() as u32).await?;
    s.write_all(&bytes).await?;
    Ok(())
}
pub async fn read_frame<R: AsyncRead + Unpin>(s: &mut R) -> Result<(u32, Value)> {
    let opcode = s.read_u32_le().await?;
    let len = s.read_u32_le().await?;
    ensure!(len <= 1024 * 1024, "Discord RPC frame exceeds 1 MiB");
    let mut bytes = vec![0; len as usize];
    s.read_exact(&mut bytes).await?;
    Ok((opcode, serde_json::from_slice(&bytes)?))
}
async fn command<W: AsyncWrite + Unpin>(s: &mut W, cmd: &str, args: Value) -> Result<String> {
    let nonce = crate::model::id();
    write_frame(s, 1, &json!({"cmd":cmd,"args":args,"nonce":nonce})).await?;
    Ok(nonce)
}
async fn response<S: AsyncRead + AsyncWrite + Unpin>(
    s: &mut S,
    cmd: &str,
    nonce: &str,
) -> Result<Value> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (opcode, value) = read_frame(s).await?;
            if opcode == 3 {
                write_frame(s, 4, &value).await?;
                continue;
            }
            ensure!(opcode != 2, "Discord closed RPC during authentication");
            if opcode == 1 && value["nonce"] == nonce && value["cmd"] == cmd {
                ensure!(
                    value["evt"] != "ERROR",
                    "Discord rejected authentication. Check RPC permissions"
                );
                return Ok(value);
            }
        }
    })
    .await?
}
async fn subscribe<W: AsyncWrite + Unpin>(
    s: &mut W,
    cmd: &str,
    evt: &str,
    args: Value,
) -> Result<()> {
    write_frame(
        s,
        1,
        &json!({"cmd":cmd,"evt":evt,"args":args,"nonce":crate::model::id()}),
    )
    .await
}
pub async fn run(e: Engine) {
    let mut delay = 1;
    loop {
        if e.live.read().unwrap().settings.discord_app.is_empty() {
            e.connection("not_configured");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }
        let token = match access(&e).await {
            Ok(v) => v,
            Err(_) => {
                e.connection("authorization_required");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        e.connection("connecting");
        match session(&e, &token).await {
            Ok(()) => delay = 1,
            Err(err) => {
                tracing::warn!(error=%err,"Discord RPC disconnected");
                e.connection("reconnecting");
            }
        }
        tokio::time::sleep(Duration::from_secs(delay)).await;
        delay = (delay * 2).min(30);
    }
}
async fn session(e: &Engine, token: &str) -> Result<()> {
    let app = e.live.read().unwrap().settings.discord_app.clone();
    let mut s = connect().await?;
    write_frame(&mut s, 0, &json!({"v":1,"client_id":app})).await?;
    let (opcode, v) = tokio::time::timeout(Duration::from_secs(10), read_frame(&mut s)).await??;
    ensure!(
        opcode == 1 && v["evt"] == "READY",
        "Discord rejected handshake"
    );
    let nonce = command(&mut s, "AUTHENTICATE", json!({"access_token":token})).await?;
    response(&mut s, "AUTHENTICATE", &nonce).await?;
    subscribe(&mut s, "SUBSCRIBE", "VOICE_CHANNEL_SELECT", json!({})).await?;
    command(&mut s, "GET_SELECTED_VOICE_CHANNEL", json!({})).await?;
    let (mut read, mut write) = tokio::io::split(s);
    let (tx, mut rx) = tokio::sync::mpsc::channel(128);
    let reader = ReaderTask(tokio::spawn(async move {
        loop {
            let frame = read_frame(&mut read).await;
            let failed = frame.is_err();
            if tx.send(frame).await.is_err() || failed {
                break;
            }
        }
    }));
    let result = async {
        let mut heartbeat = tokio::time::interval(Duration::from_secs(20));
        let mut subscribed: Option<String> = None;
        loop {
            tokio::select! {
                frame = rx.recv() => {
                    let (opcode, value) = frame.context("Discord closed the connection")??;
                    if opcode == 3 {
                        write_frame(&mut write, 4, &value).await?;
                        continue;
                    }
                    if opcode == 2 { bail!("Discord closed RPC"); }
                    if value["evt"] == "ERROR" {
                        bail!("Discord RPC request failed ({}). Check RPC voice access", value["data"]["code"]);
                    }
                    if value["cmd"] == "GET_SELECTED_VOICE_CHANNEL" || value["evt"] == "VOICE_CHANNEL_SELECT" {
                        let selected = value["data"]["id"].as_str()
                            .or_else(|| value["data"]["channel_id"].as_str()).map(str::to_owned);
                        let pinned = e.live.read().unwrap().settings.pinned_channel.clone();
                        let channel = pinned.or(selected);
                        if subscribed != channel {
                            for event in ["SPEAKING_START", "SPEAKING_STOP", "VOICE_STATE_CREATE", "VOICE_STATE_UPDATE", "VOICE_STATE_DELETE"] {
                                if let Some(old) = &subscribed {
                                    subscribe(&mut write, "UNSUBSCRIBE", event, json!({"channel_id":old})).await?;
                                }
                                if let Some(new) = &channel {
                                    subscribe(&mut write, "SUBSCRIBE", event, json!({"channel_id":new})).await?;
                                }
                            }
                            subscribed = channel.clone();
                            {
                                let mut runtime = e.runtime.write().unwrap();
                                runtime.channel = channel.clone();
                                runtime.channel_name.clear();
                                for presence in runtime.users.values_mut() {
                                    presence.present = false;
                                    presence.speaking = false;
                                }
                            }
                            if let Some(channel) = &channel {
                                command(&mut write, "GET_CHANNEL", json!({"channel_id":channel})).await?;
                            }
                        }
                        if value["data"]["voice_states"].is_array() { populate(e, &value["data"]); }
                        e.connection("connected");
                        e.notify();
                    }
                    if value["cmd"] == "GET_CHANNEL" {
                        populate(e, &value["data"]);
                        e.notify();
                    }
                    if let Some(event) = value["evt"].as_str() { handle_event(e, event, &value["data"]); }
                }
                _ = heartbeat.tick() => {
                    write_frame(&mut write, 3, &json!({"nonce":crate::model::id()})).await?;
                }
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    }.await;
    drop(reader);
    result
}
struct ReaderTask(tokio::task::JoinHandle<()>);
impl Drop for ReaderTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
fn person(v: &Value) -> Option<Person> {
    let user = &v["user"];
    let id = user["id"].as_str()?.to_owned();
    if id.is_empty() || id.len() > 32 || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let name = v["nick"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| user["global_name"].as_str())
        .or_else(|| user["username"].as_str())
        .unwrap_or("Guest")
        .to_owned();
    let avatar = user["avatar"]
        .as_str()
        .filter(|s| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .map(|hash| format!("https://cdn.discordapp.com/avatars/{id}/{hash}.png?size=256"));
    Some(Person {
        id,
        name,
        avatar,
        character: None,
    })
}
fn populate(e: &Engine, data: &Value) {
    let mut r = e.runtime.write().unwrap();
    r.channel_name = data["name"].as_str().unwrap_or("Discord").into();
    let mut current = BTreeMap::new();
    if let Some(people) = data["voice_states"].as_array() {
        for v in people {
            if let Some(p) = person(v) {
                let old = r.users.get(&p.id).cloned().unwrap_or_default();
                current.insert(
                    p.id.clone(),
                    Presence {
                        present: true,
                        muted: v["mute"].as_bool().unwrap_or(false)
                            || v["voice_state"]["self_mute"].as_bool().unwrap_or(false),
                        ..old
                    },
                );
                r.discovered.insert(p.id.clone(), p);
            }
        }
    }
    for (id, p) in r.users.iter_mut() {
        if !current.contains_key(id) {
            p.present = false;
            p.speaking = false;
        }
    }
    r.users.extend(current);
}
pub fn handle_event(e: &Engine, event: &str, data: &Value) {
    let mut r = e.runtime.write().unwrap();
    match event {
        "SPEAKING_START" | "SPEAKING_STOP" => {
            if let Some(id) = data["user_id"].as_str()
                && let Some(p) = r.users.get_mut(id)
            {
                p.speaking = event == "SPEAKING_START";
            }
        }
        "VOICE_STATE_CREATE" | "VOICE_STATE_UPDATE" => {
            if let Some(p) = person(data) {
                let presence = r.users.entry(p.id.clone()).or_default();
                presence.present = true;
                presence.muted = data["mute"].as_bool().unwrap_or(false)
                    || data["voice_state"]["self_mute"].as_bool().unwrap_or(false)
                    || data["voice_state"]["mute"].as_bool().unwrap_or(false);
                r.discovered.insert(p.id.clone(), p);
            }
        }
        "VOICE_STATE_DELETE" => {
            if let Some(id) = data["user"]["id"]
                .as_str()
                .or_else(|| data["user_id"].as_str())
                && let Some(p) = r.users.get_mut(id)
            {
                p.present = false;
                p.speaking = false;
            }
        }
        _ => return,
    };
    drop(r);
    e.notify();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn authentication_waits_for_its_reply_and_answers_ping() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let peer = tokio::spawn(async move {
            write_frame(&mut server, 3, &json!({"ping":true}))
                .await
                .unwrap();
            assert_eq!(
                read_frame(&mut server).await.unwrap(),
                (4, json!({"ping":true}))
            );
            write_frame(
                &mut server,
                1,
                &json!({"cmd":"AUTHENTICATE","nonce":"another-request"}),
            )
            .await
            .unwrap();
            write_frame(
                &mut server,
                1,
                &json!({"cmd":"AUTHENTICATE","nonce":"ours","data":{"user":{"id":"123"}}}),
            )
            .await
            .unwrap();
        });
        let reply = response(&mut client, "AUTHENTICATE", "ours").await.unwrap();
        assert_eq!(reply["data"]["user"]["id"], "123");
        peer.await.unwrap();
    }
    #[tokio::test]
    async fn framing_roundtrips_and_rejects_oversized_frames() {
        let (mut a, mut b) = tokio::io::duplex(4096);
        let v = json!({"evt":"SPEAKING_START","data":{"user_id":"123"}});
        write_frame(&mut a, 1, &v).await.unwrap();
        assert_eq!(read_frame(&mut b).await.unwrap(), (1, v));
        a.write_u32_le(1).await.unwrap();
        a.write_u32_le(1024 * 1024 + 1).await.unwrap();
        assert!(read_frame(&mut b).await.is_err());
    }
    #[test]
    fn disconnect_resets_speaking_and_preserves_manual_controls() {
        let dir = tempfile::tempdir().unwrap();
        let s = std::sync::Arc::new(crate::store::Store::open(dir.path().into()).unwrap());
        let (e, _) = Engine::new(s).unwrap();
        handle_event(
            &e,
            "VOICE_STATE_CREATE",
            &json!({"user":{"id":"123","username":"guest"}}),
        );
        handle_event(&e, "SPEAKING_START", &json!({"user_id":"123"}));
        e.connection("connected");
        assert!(e.runtime.read().unwrap().users["123"].speaking);
        e.live_action("123", "happy");
        e.connection("reconnecting");
        assert!(!e.runtime.read().unwrap().users["123"].speaking);
        assert_eq!(
            e.runtime.read().unwrap().users["123"].expression.as_deref(),
            Some("happy")
        );
    }
}
