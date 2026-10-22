//! OpenAI's public-client Sign in with ChatGPT flow. No Codex credentials are read.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use reqwest::blocking::Client;
use ring::{
    digest,
    rand::{SecureRandom, SystemRandom},
    signature,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const ISSUER: &str = "https://auth.openai.com";
const RESOURCE: &str = "https://api.openai.com/v1";
const TOKEN: &str = "https://auth.openai.com/api/accounts/oauth/token";
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
pub const USAGE_URL: &str = "https://chatgpt.com/settings/usage";

#[derive(Clone)]
pub struct Store {
    root: PathBuf,
}

#[derive(Default, Serialize, Deserialize)]
struct Database {
    host: String,
    active: Option<String>,
    accounts: Vec<Account>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Account {
    client_id: String,
    subject: String,
    email: String,
    access_token: String,
    refresh_token: String,
    id_token: String,
    scopes: Vec<String>,
    expires_at: u64,
    #[serde(default)]
    earliest_refresh_at: u64,
    #[serde(default)]
    welcomed: bool,
    model: Option<String>,
}

impl Account {
    fn enabled(&self) -> bool {
        !self.access_token.is_empty()
            && self.scopes.iter().any(|s| s == "chatgpt.tokens.use.direct")
    }
}

#[derive(Clone)]
pub struct Summary {
    pub client_id: String,
    pub email: String,
    pub active: bool,
    pub connected: bool,
    pub plan: bool,
}

#[derive(Clone)]
pub struct Model {
    pub slug: String,
    pub name: String,
}

pub enum Event {
    Waiting(String),
    Finished(Result<Authorized, String>),
}

pub struct Authorized {
    account: Account,
    welcome: bool,
}

pub struct Connection {
    pub client_id: String,
    pub plan: bool,
    pub welcome: bool,
    pub model: Option<String>,
}

pub struct Login {
    pub receiver: mpsc::Receiver<Event>,
    cancel: Arc<AtomicBool>,
}
impl Drop for Login {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn random() -> Result<String, String> {
    let mut bytes = [0; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "Secure randomness is unavailable")?;
    Ok(B64.encode(bytes))
}
fn host_id() -> Result<String, String> {
    let mut bytes = [0; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "Secure randomness is unavailable")?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}
fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(45))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Kaiser/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| "Could not initialize the ChatGPT connection".into())
}
fn response_json(response: reqwest::blocking::Response) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        // Do not surface response bodies: token endpoints can contain sensitive data.
        return Err(format!(
            "ChatGPT returned HTTP {}. Sign in again or check your connection.",
            status.as_u16()
        ));
    }
    serde_json::from_reader(response.take(2 * 1024 * 1024))
        .map_err(|_| "Invalid ChatGPT response".into())
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn lock(&self) -> Result<File, String> {
        fs::create_dir_all(&self.root)
            .map_err(|e| format!("Cannot create account storage: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(self.root.join("session.lock"))
            .map_err(|e| e.to_string())?;
        file.try_lock().map_err(|_| {
            "Another Kaiser process is updating this ChatGPT session. Try again.".to_owned()
        })?;
        Ok(file)
    }
    fn read(&self) -> Result<Database, String> {
        match fs::read(self.root.join("accounts.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|_| "Cannot read ChatGPT account storage".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Database::default()),
            Err(_) => Err("Cannot read ChatGPT account storage".into()),
        }
    }
    fn save(&self, db: &Database) -> Result<(), String> {
        let path = self.root.join(format!("{}.tmp", random()?));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| {
            let mut file = options
                .open(&path)
                .map_err(|_| "Cannot save ChatGPT account storage")?;
            serde_json::to_writer(&mut file, db)
                .map_err(|_| "Cannot serialize ChatGPT account storage")?;
            file.sync_all()
                .map_err(|_| "Cannot sync ChatGPT account storage")?;
            fs::rename(&path, self.root.join("accounts.json"))
                .map_err(|_| "Cannot replace ChatGPT account storage")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(path);
        }
        result
    }
    pub fn summaries(&self) -> Result<Vec<Summary>, String> {
        let db = self.read()?;
        Ok(db
            .accounts
            .iter()
            .map(|a| Summary {
                client_id: a.client_id.clone(),
                email: a.email.clone(),
                active: db.active.as_deref() == Some(&a.client_id),
                connected: !a.access_token.is_empty(),
                plan: a.enabled(),
            })
            .collect())
    }
    pub fn current(&self) -> Result<Option<Connection>, String> {
        let db = self.read()?;
        Ok(db
            .accounts
            .iter()
            .find(|a| db.active.as_deref() == Some(&a.client_id) && !a.access_token.is_empty())
            .map(|a| Connection {
                client_id: a.client_id.clone(),
                plan: a.enabled(),
                welcome: false,
                model: a.model.clone(),
            }))
    }
    pub fn login(&self, account: Option<String>) -> Login {
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let store = self.clone();
        std::thread::spawn(move || {
            let result = store.authorize(account, &worker_cancel, &sender);
            let _ = sender.send(Event::Finished(result));
        });
        Login { receiver, cancel }
    }
    fn authorize(
        &self,
        account: Option<String>,
        cancel: &AtomicBool,
        sender: &mpsc::Sender<Event>,
    ) -> Result<Authorized, String> {
        // Release the disk lock during browser interaction; merge only after validation.
        let (host, previous) = {
            let _lock = self.lock()?;
            let mut db = self.read()?;
            if db.host.is_empty() {
                db.host = host_id()?;
                self.save(&db)?;
            }
            let previous = account
                .as_ref()
                .map(|id| {
                    db.accounts
                        .iter()
                        .find(|a| &a.client_id == id)
                        .cloned()
                        .ok_or("Saved ChatGPT account not found")
                })
                .transpose()?;
            (db.host, previous)
        };
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|_| "Cannot start the local sign-in callback")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "Cannot configure the sign-in callback")?;
        let redirect = format!(
            "http://127.0.0.1:{}/auth/callback",
            listener
                .local_addr()
                .map_err(|_| "Cannot read callback address")?
                .port()
        );
        let state = random()?;
        let nonce = random()?;
        let verifier = random()?;
        let challenge = B64.encode(digest::digest(&digest::SHA256, verifier.as_bytes()));
        let mut url = reqwest::Url::parse(&format!("{ISSUER}/api/accounts/authorize")).unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            pairs.extend_pairs([
                (
                    "client_id",
                    previous
                        .as_ref()
                        .map(|a| a.client_id.as_str())
                        .unwrap_or("dynamic_agent_client"),
                ),
                ("ext_agent_host_id", host.as_str()),
                ("response_type", "code"),
                ("redirect_uri", &redirect),
                ("scope", SCOPE),
                ("resource", RESOURCE),
                ("state", &state),
                ("nonce", &nonce),
                ("code_challenge_method", "S256"),
                ("code_challenge", &challenge),
            ]);
            if let Some(a) = &previous {
                pairs.append_pair("login_hint", &a.email);
            } else {
                pairs.append_pair("agent_name_hint", "Kaiser");
            }
        }
        let _ = sender.send(Event::Waiting(url.to_string()));
        let _ = open_browser(url.as_str());
        let deadline = Instant::now() + Duration::from_secs(300);
        let (code, issued) = loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Sign-in cancelled".into());
            }
            if Instant::now() > deadline {
                return Err("ChatGPT sign-in timed out".into());
            }
            match listener.accept() {
                Ok((mut socket, _)) => {
                    socket.set_read_timeout(Some(Duration::from_secs(2))).ok();
                    socket.set_write_timeout(Some(Duration::from_secs(2))).ok();
                    let mut first = String::new();
                    let read = BufReader::new((&socket).take(16384)).read_line(&mut first);
                    let target = first.split_whitespace().nth(1).unwrap_or("");
                    let callback = read
                        .ok()
                        .filter(|_| first.starts_with("GET "))
                        .and_then(|_| {
                            parse_callback(
                                target,
                                &state,
                                previous.as_ref().map(|a| a.client_id.as_str()),
                            )
                            .ok()
                        });
                    let valid = callback.is_some();
                    let body = if valid {
                        "Kaiser received the sign-in response. You can return to Kaiser."
                    } else {
                        "Invalid sign-in callback. Return to Kaiser and try again."
                    };
                    let _ = write!(
                        socket,
                        "HTTP/1.1 {}\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                        if valid { "200 OK" } else { "400 Bad Request" },
                        body.len(),
                        body
                    );
                    if let Some(result) = callback {
                        break result?;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                Err(_) => return Err("Sign-in callback failed".into()),
            }
        };
        let http = client()?;
        let tokens = response_json(
            http.post(TOKEN)
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("client_id", &issued),
                    ("code", &code),
                    ("code_verifier", &verifier),
                    ("redirect_uri", &redirect),
                    ("resource", RESOURCE),
                ])
                .send()
                .map_err(|_| "ChatGPT token exchange failed")?,
        )?;
        let identity = verify_identity(
            &http,
            tokens["id_token"]
                .as_str()
                .ok_or("Missing ChatGPT identity token")?,
            &issued,
            Some(&nonce),
        )?;
        if let Some(old) = &previous
            && old.subject != identity.subject
        {
            return Err("The signed-in identity differs from the selected account".into());
        }
        let mut account = Account {
            client_id: issued.clone(),
            subject: identity.subject,
            email: identity.email,
            access_token: String::new(),
            refresh_token: String::new(),
            id_token: String::new(),
            scopes: vec![],
            expires_at: 0,
            earliest_refresh_at: 0,
            welcomed: previous.as_ref().is_some_and(|a| a.welcomed),
            model: previous.and_then(|a| a.model),
        };
        apply_tokens(&mut account, &tokens)?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Sign-in cancelled".into());
        }
        let welcome = account.enabled() && !account.welcomed;
        Ok(Authorized { account, welcome })
    }
    pub fn commit(&self, authorized: Authorized) -> Result<Connection, String> {
        let Authorized { account, welcome } = authorized;
        let issued = account.client_id.clone();
        let result = Connection {
            client_id: issued.clone(),
            plan: account.enabled(),
            welcome,
            model: account.model.clone(),
        };
        let _lock = self.lock()?;
        let mut db = self.read()?;
        if let Some(old) = db.accounts.iter_mut().find(|a| a.client_id == issued) {
            if old.subject != account.subject {
                return Err("The saved ChatGPT account identity changed".into());
            }
            *old = account;
        } else {
            db.accounts.push(account);
        }
        db.active = Some(issued);
        self.save(&db)?;
        Ok(result)
    }
    pub fn welcome_seen(&self) -> Result<(), String> {
        let _lock = self.lock()?;
        let mut db = self.read()?;
        if let Some(a) = db
            .accounts
            .iter_mut()
            .find(|a| Some(&a.client_id) == db.active.as_ref())
        {
            a.welcomed = true;
        }
        self.save(&db)
    }
    pub fn select(&self, id: &str) -> Result<Connection, String> {
        let _lock = self.lock()?;
        let mut db = self.read()?;
        if !db.accounts.iter().any(|a| a.client_id == id) {
            return Err("ChatGPT account not found".into());
        }
        db.active = Some(id.to_owned());
        self.save(&db)?;
        self.current()?
            .ok_or("Sign in to this account again".into())
    }
    pub fn set_model(&self, model: &str) -> Result<(), String> {
        let _lock = self.lock()?;
        let mut db = self.read()?;
        let account = db
            .accounts
            .iter_mut()
            .find(|a| Some(&a.client_id) == db.active.as_ref())
            .ok_or("Sign in with ChatGPT first")?;
        account.model = Some(model.to_owned());
        self.save(&db)
    }
    pub fn access_token(&self, id: &str) -> Result<String, String> {
        // The OS lock serializes refresh-token rotation across threads and processes.
        let _lock = self.lock()?;
        let mut db = self.read()?;
        let account = db
            .accounts
            .iter_mut()
            .find(|a| a.client_id == id)
            .ok_or("ChatGPT account not found")?;
        if !account.enabled() {
            return Err("ChatGPT plan usage is disabled. Sign in and grant plan usage.".into());
        }
        if account.expires_at <= now() + 60 && account.earliest_refresh_at <= now() {
            if account.refresh_token.is_empty() {
                return Err("ChatGPT session expired. Sign in again.".into());
            }
            let http = client()?;
            let tokens = response_json(
                http.post(TOKEN)
                    .form(&[
                        ("grant_type", "refresh_token"),
                        ("client_id", id),
                        ("refresh_token", account.refresh_token.as_str()),
                        ("resource", RESOURCE),
                    ])
                    .send()
                    .map_err(|_| "ChatGPT session refresh failed")?,
            )?;
            if let Some(token) = tokens["id_token"].as_str() {
                let identity = verify_identity(&http, token, id, None)?;
                if identity.subject != account.subject {
                    return Err("ChatGPT refreshed identity did not match".into());
                }
            }
            apply_tokens(account, &tokens)?;
            self.save(&db)?;
        }
        let account = db.accounts.iter().find(|a| a.client_id == id).unwrap();
        if !account.enabled() {
            return Err("ChatGPT plan usage is no longer authorized".into());
        }
        Ok(account.access_token.clone())
    }
    pub fn models(&self, id: &str) -> Result<Vec<Model>, String> {
        let token = self.access_token(id)?;
        let data = response_json(
            client()?
                .get(format!("{RESOURCE}/models"))
                .bearer_auth(token)
                .send()
                .map_err(|_| "Could not load ChatGPT models")?,
        )?;
        let models: Vec<_> = data["models"]
            .as_array()
            .ok_or("Invalid ChatGPT model catalog")?
            .iter()
            .filter(|m| m["visibility"] == "list")
            .filter_map(|m| {
                Some(Model {
                    slug: m["slug"].as_str()?.into(),
                    name: m["display_name"].as_str()?.into(),
                })
            })
            .collect();
        if models.is_empty() {
            Err("No models are available for this ChatGPT account".into())
        } else {
            Ok(models)
        }
    }
    pub fn logout(&self) -> Result<String, String> {
        let _lock = self.lock()?;
        let mut db = self.read()?;
        let Some(account) = db
            .accounts
            .iter_mut()
            .find(|a| Some(&a.client_id) == db.active.as_ref())
        else {
            return Ok("Signed out".into());
        };
        let remote = if account.refresh_token.is_empty() {
            true
        } else {
            (|| -> Result<bool, String> {
                let http = client()?;
                let discovery = response_json(
                    http.get(format!("{ISSUER}/.well-known/openid-configuration"))
                        .send()
                        .map_err(|_| "Discovery failed")?,
                )?;
                let endpoint = discovery["revocation_endpoint"]
                    .as_str()
                    .ok_or("Missing revocation endpoint")?;
                trusted_endpoint(endpoint)?;
                for attempt in 0..3 {
                    let result = http
                        .post(endpoint)
                        .form(&[
                            ("token", account.refresh_token.as_str()),
                            ("token_type_hint", "refresh_token"),
                            ("client_id", account.client_id.as_str()),
                        ])
                        .send();
                    match result {
                        Ok(response) if response.status().as_u16() == 200 => return Ok(true),
                        Ok(response) if !response.status().is_server_error() => return Ok(false),
                        _ => std::thread::sleep(Duration::from_millis(100 * (1 << attempt))),
                    }
                }
                Ok(false)
            })()
            .unwrap_or(false)
        };
        account.access_token.clear();
        account.refresh_token.clear();
        account.id_token.clear();
        account.scopes.clear();
        account.expires_at = 0;
        db.active = None;
        self.save(&db)?;
        Ok(if remote {
            "Signed out of ChatGPT".into()
        } else {
            "Signed out locally. Remote revocation was not confirmed; disconnect Kaiser in ChatGPT Settings.".into()
        })
    }
}

fn trusted_endpoint(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "Invalid OpenAI endpoint")?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("auth.openai.com")
        || parsed.port().is_some()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("Untrusted OpenAI endpoint".into());
    }
    Ok(())
}

/// The outer error rejects unsolicited callbacks; the inner error ends a valid declined flow.
fn parse_callback(
    target: &str,
    state: &str,
    previous: Option<&str>,
) -> Result<Result<(String, String), String>, String> {
    if !target.starts_with("/auth/callback?") {
        return Err("Invalid callback path".into());
    }
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| "Invalid callback")?;
    let mut params = std::collections::HashMap::new();
    for (key, value) in url.query_pairs() {
        if params
            .insert(key.into_owned(), value.into_owned())
            .is_some()
        {
            return Err("Duplicate callback parameter".into());
        }
    }
    if params.get("state").map(String::as_str) != Some(state) {
        return Err("Invalid callback state".into());
    }
    if params.contains_key("error") {
        return Ok(Err("ChatGPT sign-in was declined or failed".into()));
    }
    let issued = match (previous, params.get("client_id")) {
        (Some(old), Some(new)) if old != new => return Err("ChatGPT client ID changed".into()),
        (Some(old), _) => old.to_owned(),
        (None, Some(id)) if id.starts_with("oaiapp_") => id.clone(),
        _ => return Err("Missing issued ChatGPT client ID".into()),
    };
    let code = params
        .get("code")
        .filter(|s| !s.is_empty())
        .ok_or("Missing authorization code")?
        .clone();
    Ok(Ok((code, issued)))
}

struct Identity {
    subject: String,
    email: String,
}

fn verify_identity(
    http: &Client,
    token: &str,
    audience: &str,
    nonce: Option<&str>,
) -> Result<Identity, String> {
    let jwks = response_json(
        http.get(format!("{ISSUER}/.well-known/jwks.json"))
            .send()
            .map_err(|_| "Could not retrieve OpenAI signing keys")?,
    )?;
    verify_signed_identity(token, &jwks, audience, nonce)
}

fn verify_signed_identity(
    token: &str,
    jwks: &Value,
    audience: &str,
    nonce: Option<&str>,
) -> Result<Identity, String> {
    let parts: Vec<_> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid ChatGPT identity token".into());
    }
    let decode = |s| {
        B64.decode(s)
            .map_err(|_| "Invalid identity encoding".to_owned())
    };
    let header: Value =
        serde_json::from_slice(&decode(parts[0])?).map_err(|_| "Invalid identity header")?;
    if header["alg"] != "RS256" {
        return Err("Unsupported identity signature algorithm".into());
    }
    if header.get("crit").is_some() {
        return Err("Unsupported critical identity header".into());
    }
    let kid = header["kid"]
        .as_str()
        .ok_or("Missing identity signing key")?;
    let key = jwks["keys"]
        .as_array()
        .and_then(|keys| {
            keys.iter().find(|k| {
                k["kid"] == kid
                    && k["kty"] == "RSA"
                    && k["alg"].as_str().is_none_or(|alg| alg == "RS256")
                    && k["use"].as_str().is_none_or(|usage| usage == "sig")
            })
        })
        .ok_or("Unknown OpenAI signing key")?;
    let n = decode(key["n"].as_str().ok_or("Missing RSA modulus")?)?;
    let e = decode(key["e"].as_str().ok_or("Missing RSA exponent")?)?;
    signature::RsaPublicKeyComponents { n: &n, e: &e }
        .verify(
            &signature::RSA_PKCS1_2048_8192_SHA256,
            format!("{}.{}", parts[0], parts[1]).as_bytes(),
            &decode(parts[2])?,
        )
        .map_err(|_| "ChatGPT identity signature is invalid")?;
    let claims: Value =
        serde_json::from_slice(&decode(parts[1])?).map_err(|_| "Invalid identity claims")?;
    validate_claims(&claims, audience, nonce)
}

fn validate_claims(
    claims: &Value,
    audience: &str,
    nonce: Option<&str>,
) -> Result<Identity, String> {
    let aud = &claims["aud"];
    let audience_valid = aud.as_str() == Some(audience)
        || aud
            .as_array()
            .is_some_and(|items| items.iter().any(|v| v.as_str() == Some(audience)));
    if claims["iss"] != ISSUER
        || !audience_valid
        || claims["exp"].as_u64().is_none_or(|exp| exp <= now())
        || claims["iat"].as_u64().is_none_or(|iat| iat > now() + 5)
        || claims["nbf"].as_u64().is_some_and(|nbf| nbf > now() + 5)
        || nonce.is_some_and(|nonce| claims["nonce"].as_str() != Some(nonce))
        || claims["azp"].as_str().is_some_and(|azp| azp != audience)
        || (aud.as_array().is_some_and(|items| items.len() > 1)
            && claims["azp"].as_str() != Some(audience))
    {
        return Err("ChatGPT identity validation failed".into());
    }
    let subject = claims["sub"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("Missing ChatGPT identity")?
        .to_owned();
    let email = claims["email"]
        .as_str()
        .unwrap_or("ChatGPT account")
        .to_owned();
    Ok(Identity { subject, email })
}

fn apply_tokens(account: &mut Account, tokens: &Value) -> Result<(), String> {
    if !tokens["token_type"]
        .as_str()
        .is_some_and(|s| s.eq_ignore_ascii_case("Bearer"))
    {
        return Err("Invalid ChatGPT token type".into());
    }
    let access = tokens["access_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("Missing ChatGPT access token")?;
    let expires = tokens["expires_in"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= 86400)
        .ok_or("Invalid ChatGPT token expiration")?;
    account.access_token = access.into();
    account.expires_at = now() + expires;
    account.earliest_refresh_at = tokens["earliest_refresh_at"].as_u64().unwrap_or(0);
    if let Some(token) = tokens["refresh_token"].as_str() {
        account.refresh_token = token.into();
    }
    if let Some(token) = tokens["id_token"].as_str() {
        account.id_token = token.into();
    }
    if let Some(scope) = tokens["scope"].as_str() {
        account.scopes = scope.split_whitespace().map(str::to_owned).collect();
    }
    Ok(())
}

pub fn open_browser(url: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not open the browser. Open the displayed URL manually.".to_owned())?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn account(id: &str) -> Account {
        Account {
            client_id: id.into(),
            subject: "subject".into(),
            email: "example@example.invalid".into(),
            access_token: "synthetic-access".into(),
            refresh_token: String::new(),
            id_token: "synthetic-id".into(),
            scopes: vec!["chatgpt.tokens.use.direct".into()],
            expires_at: now() + 3600,
            earliest_refresh_at: 0,
            welcomed: false,
            model: None,
        }
    }

    #[test]
    fn callbacks_reject_wrong_state_duplicates_missing_or_changed_client_and_allow_decline() {
        let host = host_id().unwrap();
        assert!(host.starts_with("urn:uuid:"));
        assert_eq!(host.len(), 45);
        assert_eq!(host.chars().nth(23), Some('4'));
        let callback = "/auth/callback?state=expected&code=code&client_id=oaiapp_test";
        assert_eq!(
            parse_callback(callback, "expected", None).unwrap().unwrap(),
            ("code".into(), "oaiapp_test".into())
        );
        assert!(parse_callback(callback, "wrong", None).is_err());
        assert!(parse_callback(callback, "expected", Some("oaiapp_other")).is_err());
        assert!(parse_callback(&format!("{callback}&state=expected"), "expected", None).is_err());
        assert!(parse_callback("/callback?state=expected&code=code", "expected", None).is_err());
        assert!(
            parse_callback("/auth/callback?state=expected&code=code", "expected", None).is_err()
        );
        assert!(
            parse_callback(
                "/auth/callback?state=expected&code=code&client_id=dynamic_agent_client",
                "expected",
                None
            )
            .is_err()
        );
        assert!(
            parse_callback(
                "/auth/callback?state=expected&error=access_denied",
                "expected",
                None
            )
            .unwrap()
            .is_err()
        );
        assert!(
            parse_callback(
                "/auth/callback?state=expected&code=code",
                "expected",
                Some("oaiapp_test")
            )
            .unwrap()
            .is_ok()
        );
    }

    #[test]
    fn dropping_a_login_cancels_it_without_committing_credentials() {
        let (_sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let login = Login {
            receiver,
            cancel: cancel.clone(),
        };
        drop(login);
        assert!(cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn signed_identity_requires_valid_signature_issuer_audience_nonce_and_expiry() {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/fixtures/chatgpt-identity.json")).unwrap();
        let token = fixture["token"].as_str().unwrap();
        let keys = &fixture["jwks"];
        assert_eq!(
            verify_signed_identity(token, keys, "oaiapp_fixture", Some("test-nonce"))
                .unwrap()
                .subject,
            "test-subject"
        );
        assert!(verify_signed_identity(token, keys, "oaiapp_other", Some("test-nonce")).is_err());
        assert!(verify_signed_identity(token, keys, "oaiapp_fixture", Some("wrong")).is_err());
        let mut parts: Vec<String> = token.split('.').map(str::to_owned).collect();
        let mut claims: Value = serde_json::from_slice(&B64.decode(&parts[1]).unwrap()).unwrap();
        claims["sub"] = json!("tampered");
        parts[1] = B64.encode(serde_json::to_vec(&claims).unwrap());
        assert!(
            verify_signed_identity(&parts.join("."), keys, "oaiapp_fixture", Some("test-nonce"))
                .is_err()
        );
        assert!(
            verify_signed_identity(token, &json!({"keys": []}), "oaiapp_fixture", None).is_err()
        );
        claims["exp"] = json!(1);
        assert!(validate_claims(&claims, "oaiapp_fixture", None).is_err());
        claims["exp"] = json!(now() + 60);
        claims["iss"] = json!("https://example.invalid");
        assert!(validate_claims(&claims, "oaiapp_fixture", None).is_err());
        assert!(trusted_endpoint("http://auth.openai.com/token").is_err());
        assert!(trusted_endpoint("https://auth.openai.com.example.invalid/token").is_err());
    }

    #[test]
    fn accounts_keep_separate_credentials_rotate_tokens_and_sign_out_without_losing_registration() {
        let root = std::env::temp_dir().join(format!("kaiser-auth-{}", random().unwrap()));
        let store = Store::new(root.clone());
        assert!(store.current().unwrap().is_none());
        let _lock = store.lock().unwrap();
        assert!(store.lock().is_err());
        store
            .save(&Database {
                host: "stable-host".into(),
                active: Some("oaiapp_first".into()),
                accounts: vec![account("oaiapp_first"), account("oaiapp_second")],
            })
            .unwrap();
        drop(_lock);
        assert!(store.current().unwrap().unwrap().plan);
        assert_eq!(
            store.access_token("oaiapp_first").unwrap(),
            "synthetic-access"
        );
        store.select("oaiapp_second").unwrap();
        store.set_model("server-model").unwrap();
        store.welcome_seen().unwrap();
        assert_eq!(
            store.current().unwrap().unwrap().model.as_deref(),
            Some("server-model")
        );
        let db = store.read().unwrap();
        assert!(db.accounts[0].model.is_none());
        assert!(db.accounts[1].welcomed);
        let mut refreshed = account("oaiapp_first");
        apply_tokens(&mut refreshed, &json!({"token_type":"Bearer","access_token":"replacement","refresh_token":"rotated","expires_in":3600,"scope":"openid"})).unwrap();
        assert_eq!(refreshed.refresh_token, "rotated");
        assert!(!refreshed.enabled());
        assert!(
            store
                .commit(Authorized {
                    account: Account {
                        subject: "different".into(),
                        ..account("oaiapp_first")
                    },
                    welcome: false
                })
                .is_err()
        );
        assert_eq!(store.logout().unwrap(), "Signed out of ChatGPT");
        assert!(store.current().unwrap().is_none());
        let db = store.read().unwrap();
        assert_eq!(db.host, "stable-host");
        assert_eq!(db.accounts.len(), 2);
        assert!(db.accounts[1].access_token.is_empty());
        assert_eq!(db.accounts[0].access_token, "synthetic-access");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(root.join("accounts.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}
