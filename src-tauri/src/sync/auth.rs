//! Google OAuth — the consent flow, the token store and the account lookup.
//!
//! checkd never ships an OAuth client of its own: the user brings their own
//! desktop OAuth client from the Google Cloud console and the app keeps it in
//! the operating system's keyring. See `docs/google-calendar-sync-design.md`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// The scope checkd asks for — the same one adventure_list uses, so the
/// consent screen reads the same for both apps.
pub const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar";

/// The scope that lets the app show *which* account is connected. Without it
/// Google does not hand out the email address.
pub const EMAIL_SCOPE: &str = "https://www.googleapis.com/auth/userinfo.email";

/// The scopes the consent screen asks for.
pub fn scopes() -> String {
    format!("{CALENDAR_SCOPE} {EMAIL_SCOPE}")
}

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
const TOKENINFO_URL: &str = "https://oauth2.googleapis.com/tokeninfo";
const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

/// The user's own OAuth client (from the Google Cloud console).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
}

impl OAuthClient {
    /// Checks what the user pasted into the settings.
    pub fn validate(client_id: &str, client_secret: &str) -> Result<Self, String> {
        let client_id = client_id.trim();
        let client_secret = client_secret.trim();

        if client_id.is_empty() || client_secret.is_empty() {
            return Err("Both the client id and the client secret are needed.".into());
        }
        if !client_id.ends_with("apps.googleusercontent.com") {
            return Err(
                "That does not look like a Google client id — it ends in apps.googleusercontent.com."
                    .into(),
            );
        }

        Ok(OAuthClient {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        })
    }
}

/// The tokens of a connected account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    /// Missing when the tokens were refreshed — the old refresh token stays
    /// valid, so the caller keeps the one it has.
    pub refresh_token: Option<String>,
    /// When the access token expires (unix seconds).
    pub expires_at: i64,
}

impl Tokens {
    /// Whether the access token is still usable.
    pub fn is_fresh(&self) -> bool {
        self.expires_at > chrono::Utc::now().timestamp() + 60
    }
}

/// Where the secrets live.
///
/// The app uses the operating system's keyring; the tests use a fake so they
/// never touch the real one.
pub trait SecretStore: Send + Sync + std::fmt::Debug {
    fn get(&self, name: &str) -> Result<Option<String>, String>;
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;

    /// Called once the app knows where its data directory is — a store with a
    /// file fallback needs it.
    fn set_fallback_dir(&self, _dir: std::path::PathBuf) {}
}

/// The OS keyring — libsecret on Linux, the keychain on macOS, DPAPI on
/// Windows.
#[derive(Debug)]
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new(service: &str) -> Self {
        KeyringStore {
            service: service.to_string(),
        }
    }

    fn entry(&self, name: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(&self.service, name).map_err(|e| e.to_string())
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        match self.entry(name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err.to_string()),
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        self.entry(name)?
            .set_password(value)
            .map_err(|e| e.to_string())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        match self.entry(name)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

/// A secret store that keeps everything in memory — used by the tests.
#[derive(Debug, Default)]
pub struct MemoryStore {
    values: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        let values = self.values.lock().unwrap_or_else(|err| err.into_inner());
        Ok(values.get(name).cloned())
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let mut values = self.values.lock().unwrap_or_else(|err| err.into_inner());
        values.insert(name.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        let mut values = self.values.lock().unwrap_or_else(|err| err.into_inner());
        values.remove(name);
        Ok(())
    }
}

/// A file store for systems without a keyring.
///
/// The secrets end up in the app-data directory, readable by the user only —
/// `0600` for the files, `0700` for the directory.
#[derive(Debug, Clone)]
pub struct FileStore {
    dir: std::path::PathBuf,
}

impl FileStore {
    pub fn new(dir: std::path::PathBuf) -> Self {
        FileStore { dir }
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.dir.join(format!("{name}.secret"))
    }

    fn ensure_dir(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let _ = set_permissions(&self.dir, 0o700);
        Ok(())
    }
}

impl SecretStore for FileStore {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        match std::fs::read_to_string(self.path(name)) {
            Ok(value) => Ok(Some(value.trim_end().to_string())),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.to_string()),
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        self.ensure_dir()?;
        let path = self.path(name);
        std::fs::write(&path, value).map_err(|e| e.to_string())?;
        let _ = set_permissions(&path, 0o600);
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        match std::fs::remove_file(self.path(name)) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

/// The keyring — with a file fallback for systems that have no keyring
/// (a headless Linux without libsecret, for example).
#[derive(Debug)]
pub struct SafeStore {
    keyring: KeyringStore,
    fallback: std::sync::Mutex<Option<FileStore>>,
}

impl SafeStore {
    pub fn new(service: &str) -> Self {
        SafeStore {
            keyring: KeyringStore::new(service),
            fallback: std::sync::Mutex::new(None),
        }
    }

    /// The file store to fall back to, if there is one.
    fn fallback(&self) -> Option<FileStore> {
        self.fallback
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }
}

impl SecretStore for SafeStore {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        match self.keyring.get(name) {
            Ok(value) => Ok(value),
            Err(err) => match self.fallback() {
                Some(store) => store.get(name),
                None => Err(err),
            },
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        match self.keyring.set(name, value) {
            Ok(()) => Ok(()),
            Err(err) => match self.fallback() {
                Some(store) => store.set(name, value),
                None => Err(err),
            },
        }
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        match self.keyring.delete(name) {
            Ok(()) => Ok(()),
            Err(err) => match self.fallback() {
                Some(store) => store.delete(name),
                None => Err(err),
            },
        }
    }

    fn set_fallback_dir(&self, dir: std::path::PathBuf) {
        let mut fallback = self.fallback.lock().unwrap_or_else(|err| err.into_inner());
        *fallback = Some(FileStore::new(dir));
    }
}

/// Restricts a path to the given mode (a no-op outside unix).
#[cfg(unix)]
fn set_permissions(path: &std::path::Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| e.to_string())
}

/// Restricts a path to the given mode (a no-op outside unix).
#[cfg(not(unix))]
fn set_permissions(_path: &std::path::Path, _mode: u32) -> Result<(), String> {
    Ok(())
}

/// The URL the browser is sent to.
pub fn consent_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
        percent_encode(client_id),
        percent_encode(redirect_uri),
        percent_encode(&scopes()),
        percent_encode(state),
    )
}

/// Waits for the browser to come back with the authorization code.
///
/// This is the loopback half of the "installed app" flow: the browser sends
/// the code to `http://127.0.0.1:<port>/?code=…&state=…`.
pub fn wait_for_code(listener: TcpListener, expected_state: &str) -> Result<String, String> {
    let mut stream = accept_with_timeout(&listener, Duration::from_secs(300))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));

    // The request usually arrives in one piece, but it does not have to.
    let mut request = String::new();
    let mut buffer = [0u8; 4096];
    while !request.contains('\n') {
        let read = stream.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        request.push_str(&String::from_utf8_lossy(&buffer[..read]));
    }

    let target = request.lines().next().unwrap_or_default();

    let query = target.split_once('?').map(|(_, query)| query).unwrap_or("");
    let mut code = None;
    let mut state = None;
    let mut error = None;

    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        // The request line ends with the HTTP version — it is not part of the
        // value.
        let value = value.split([' ', '\r']).next().unwrap_or("");
        let value = percent_decode(value);
        match key {
            "code" => code = Some(value),
            "state" => state = Some(value),
            "error" => error = Some(value),
            _ => {}
        }
    }

    let body = match (&error, &code) {
        (Some(err), _) => format!("<h1>Sign-in failed</h1><p>{}</p>", html_escape(err)),
        (None, Some(_)) => "<h1>checkd is connected</h1><p>You can close this tab.</p>".to_string(),
        _ => "<h1>Sign-in failed</h1><p>No authorization code came back.</p>".to_string(),
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    if let Some(err) = error {
        return Err(format!("Google refused the sign-in: {err}"));
    }

    let code = code.ok_or("Google did not return an authorization code.")?;
    if state.as_deref() != Some(expected_state) {
        return Err("The sign-in could not be verified — please try again.".into());
    }

    Ok(code)
}

/// Waits for the browser to connect, but not forever.
fn accept_with_timeout(
    listener: &TcpListener,
    timeout: Duration,
) -> Result<std::net::TcpStream, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                return Ok(stream);
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if start.elapsed() > timeout {
                    return Err("The sign-in timed out — please try again.".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(err.to_string()),
        }
    }
}

/// Exchanges an authorization code for tokens.
pub async fn exchange_code(
    client: &OAuthClient,
    code: &str,
    redirect_uri: &str,
) -> Result<Tokens, String> {
    let form = [
        ("code", code.to_string()),
        ("client_id", client.client_id.clone()),
        ("client_secret", client.client_secret.clone()),
        ("redirect_uri", redirect_uri.to_string()),
        ("grant_type", "authorization_code".to_string()),
    ];

    let response = reqwest::Client::new()
        .post(TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    read_tokens(response).await
}

/// Asks Google for a fresh access token.
pub async fn refresh(client: &OAuthClient, refresh_token: &str) -> Result<Tokens, String> {
    let form = [
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("client_id", client.client_id.clone()),
        ("client_secret", client.client_secret.clone()),
    ];

    let response = reqwest::Client::new()
        .post(TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    read_tokens(response).await
}

/// Gives the access back to Google — the account can then revoke it.
pub async fn revoke(token: &str) -> Result<(), String> {
    let response = reqwest::Client::new()
        .post(REVOKE_URL)
        .form(&[("token", token.to_string())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(describe_error(&response.text().await.unwrap_or_default()))
    }
}

/// The email of the connected account.
pub async fn account_email(access_token: &str) -> Result<String, String> {
    // The token endpoint knows the email, but only when the email scope was
    // granted.
    if let Some(email) = tokeninfo_email(access_token).await? {
        return Ok(email);
    }

    // Otherwise the userinfo endpoint answers it.
    let response = reqwest::Client::new()
        .get(USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(describe_error(&response.text().await.unwrap_or_default()));
    }

    let body = response.text().await.map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    value
        .get("email")
        .and_then(|email| email.as_str())
        .map(str::to_string)
        .ok_or_else(|| "Google did not report an account.".to_string())
}

/// The email in the token info — `None` when the email scope was not granted.
async fn tokeninfo_email(access_token: &str) -> Result<Option<String>, String> {
    let response = reqwest::Client::new()
        .get(TOKENINFO_URL)
        .query(&[("access_token", access_token)])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(describe_error(&response.text().await.unwrap_or_default()));
    }

    let body = response.text().await.map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    Ok(value
        .get("email")
        .and_then(|email| email.as_str())
        .map(str::to_string))
}

/// Reads the token endpoint's answer.
async fn read_tokens(response: reqwest::Response) -> Result<Tokens, String> {
    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(describe_error(&body));
    }

    let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let access_token = value
        .get("access_token")
        .and_then(|token| token.as_str())
        .ok_or("Google did not return an access token.")?
        .to_string();
    let refresh_token = value
        .get("refresh_token")
        .and_then(|token| token.as_str())
        .map(str::to_string);
    let expires_in = value
        .get("expires_in")
        .and_then(|seconds| seconds.as_i64())
        .unwrap_or(3600);

    Ok(Tokens {
        access_token,
        refresh_token,
        expires_at: chrono::Utc::now().timestamp() + expires_in,
    })
}

/// Turns Google's error answer into something a person can read.
fn describe_error(body: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or_default();

    if let Some(message) = parsed.get("error_description").and_then(|v| v.as_str()) {
        return message.to_string();
    }
    if let Some(error) = parsed.get("error").and_then(|v| v.as_str()) {
        return match error {
            "invalid_client" => {
                "Google does not accept this client id and secret.".to_string()
            }
            "invalid_grant" => {
                "The sign-in has expired or was revoked — please try again.".to_string()
            }
            other => format!("Google refused the request: {other}"),
        };
    }
    // The API answers with a nested error object.
    if let Some(message) = parsed
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
    {
        return message.to_string();
    }
    if body.trim().is_empty() {
        "Google refused the request.".to_string()
    } else {
        body.trim().to_string()
    }
}

/// Percent-encodes everything that is not unreserved (RFC 3986).
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// The inverse of `percent_encode`, plus `+` for spaces.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        out.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            other => {
                out.push(other);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// Escapes the little HTML the loopback page shows.
fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_consent_url_carries_everything_google_needs() {
        let url = consent_url(
            "123.apps.googleusercontent.com",
            "http://127.0.0.1:5000",
            "abc-123",
        );

        assert!(url.starts_with(AUTH_URL));
        assert!(url.contains("client_id=123.apps.googleusercontent.com"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A5000"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("state=abc-123"));
        // The scope has to be encoded — it contains a colon and a slash.
        assert!(url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar"));
        // The email scope comes with it — that is how the app knows which
        // account is connected.
        assert!(url.contains(
            "scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar%20https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fuserinfo.email"
        ));
    }

    #[test]
    fn percent_encoding_round_trips() {
        for value in [
            "https://www.googleapis.com/auth/calendar",
            "http://127.0.0.1:5000",
            "abc-123",
            "a b&c=d",
        ] {
            assert_eq!(percent_decode(&percent_encode(value)), value);
        }
    }

    #[test]
    fn a_client_id_has_to_look_like_a_google_one() {
        assert!(OAuthClient::validate("", "secret").is_err());
        assert!(OAuthClient::validate("123.apps.googleusercontent.com", "").is_err());
        assert!(OAuthClient::validate("not-a-client", "secret").is_err());
        assert!(OAuthClient::validate("123.apps.googleusercontent.com", "secret").is_ok());
    }

    #[test]
    fn the_memory_store_behaves_like_a_keyring() {
        let store = MemoryStore::default();
        assert_eq!(store.get("token").unwrap(), None);
        store.set("token", "value").unwrap();
        assert_eq!(store.get("token").unwrap().as_deref(), Some("value"));
        store.delete("token").unwrap();
        assert_eq!(store.get("token").unwrap(), None);
    }

    /// The fallback for systems without a keyring keeps the secrets to
    /// themselves.
    #[test]
    fn the_file_store_keeps_the_secrets_to_itself() {
        let dir = std::env::temp_dir().join(format!("checkd-test-secrets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = FileStore::new(dir.clone());

        assert_eq!(store.get("token").unwrap(), None);
        store.set("token", "value").unwrap();
        assert_eq!(store.get("token").unwrap().as_deref(), Some("value"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("token.secret"))
                .expect("the file is there")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        store.delete("token").unwrap();
        assert_eq!(store.get("token").unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Whether the system has a keyring or not, the value comes back.
    #[test]
    fn the_safe_store_works_with_or_without_a_keyring() {
        let dir = std::env::temp_dir().join(format!("checkd-test-safe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let store = SafeStore::new("checkd-test-safe");
        store.set_fallback_dir(dir.clone());

        store.set("token", "value").expect("the secret is stored");
        assert_eq!(store.get("token").unwrap().as_deref(), Some("value"));

        store.delete("token").expect("the secret is gone");
        assert_eq!(store.get("token").unwrap(), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The browser answers on the loopback listener — that is where the
    /// authorization code comes from.
    #[test]
    fn the_loopback_listener_reads_the_code() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();

        let answer = std::thread::spawn(move || {
            let mut stream =
                std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
            stream
                .write_all(b"GET /?code=4/0Abc&state=xyz HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("write");
            stream
        });

        let code = wait_for_code(listener, "xyz").expect("the code comes back");
        assert_eq!(code, "4/0Abc");

        // The browser gets a page back.
        let mut page = String::new();
        let mut stream = answer.join().expect("thread");
        let _ = stream.read_to_string(&mut page);
        assert!(page.contains("checkd is connected"));
    }

    /// A code that comes with the wrong state is refused.
    #[test]
    fn the_loopback_listener_checks_the_state() {        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();

        std::thread::spawn(move || {
            let mut stream =
                std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
            let _ = stream.write_all(b"GET /?code=abc&state=wrong HTTP/1.1\r\n\r\n");
            let mut sink = Vec::new();
            let _ = stream.read_to_end(&mut sink);
        });

        let error = wait_for_code(listener, "xyz").expect_err("the state is checked");
        assert!(error.contains("could not be verified"), "unexpected: {error}");
    }

    /// Google reports the problem in the redirect instead of a code.
    #[test]
    fn a_refused_sign_in_is_passed_on() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();

        std::thread::spawn(move || {
            let mut stream =
                std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
            let _ = stream.write_all(b"GET /?error=access_denied&state=xyz HTTP/1.1\r\n\r\n");
            let mut sink = Vec::new();
            let _ = stream.read_to_end(&mut sink);
        });

        let error = wait_for_code(listener, "xyz").expect_err("the error comes back");
        assert!(error.contains("access_denied"), "unexpected: {error}");
    }
}
