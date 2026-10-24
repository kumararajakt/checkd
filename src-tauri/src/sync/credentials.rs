use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::sync::auth::OAuthClient;

pub const CLIENT_ID_VARS: [&str; 2] = ["CHECKD_GOOGLE_CLIENT_ID", "CLIENT_ID"];
pub const CLIENT_SECRET_VARS: [&str; 2] = ["CHECKD_GOOGLE_CLIENT_SECRET", "CLIENT_SECRET"];

pub const BUILTIN_CLIENT_ID: Option<&str> = option_env!("CHECKD_BUILTIN_CLIENT_ID");
pub const BUILTIN_CLIENT_SECRET: Option<&str> = option_env!("CHECKD_BUILTIN_CLIENT_SECRET");

pub const ENV_FILE: &str = ".env";

#[derive(Debug, Clone)]
pub enum ClientSource {
    Environment(String),
    File(PathBuf),
    Builtin,
}

impl ClientSource {
    pub fn describe(&self) -> String {
        match self {
            ClientSource::Environment(name) => format!("{name} (environment)"),
            ClientSource::File(path) => path.display().to_string(),
            ClientSource::Builtin => "built-in".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedClient {
    pub client: OAuthClient,
    pub source: ClientSource,
}

pub fn resolve_client(dirs: &[PathBuf]) -> Result<Option<ResolvedClient>, String> {
    if let Some((id, id_var)) = env_value(&CLIENT_ID_VARS) {
        let (secret, _) = env_value(&CLIENT_SECRET_VARS).ok_or_else(|| {
            format!(
                "{} is set, but the client secret is missing — set {} as well.",
                id_var, CLIENT_SECRET_VARS.join(" or ")
            )
        })?;

        let client = OAuthClient::validate(&id, &secret)?;
        return Ok(Some(ResolvedClient {
            client,
            source: ClientSource::Environment(id_var),
        }));
    }

    for dir in dirs {
        let path = dir.join(ENV_FILE);
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };

        let values = parse_env_file(&contents);
        let Some((id, _)) = file_value(&values, &CLIENT_ID_VARS) else {
            continue;
        };
        let (secret, _) = file_value(&values, &CLIENT_SECRET_VARS).ok_or_else(|| {
            format!(
                "{} has a client id, but no client secret.",
                path.display()
            )
        })?;

        let client = OAuthClient::validate(&id, &secret)?;
        return Ok(Some(ResolvedClient {
            client,
            source: ClientSource::File(path),
        }));
    }

    if let (Some(id), Some(secret)) = (BUILTIN_CLIENT_ID, BUILTIN_CLIENT_SECRET) {
        if let Ok(client) = OAuthClient::validate(id, secret) {
            return Ok(Some(ResolvedClient {
                client,
                source: ClientSource::Builtin,
            }));
        }
    }

    Ok(None)
}

pub fn search_dirs(app_data_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(dir) = app_data_dir {
        dirs.push(dir);
    }

    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.clone());
        if let Some(parent) = cwd.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    dirs
}

fn env_value(names: &[&str]) -> Option<(String, String)> {
    for name in names {
        if let Ok(value) = std::env::var(name) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some((value, (*name).to_string()));
            }
        }
    }

    None
}

fn file_value(values: &HashMap<String, String>, names: &[&str]) -> Option<(String, String)> {
    for name in names {
        if let Some(value) = values.get(*name) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some((value, (*name).to_string()));
            }
        }
    }

    None
}

fn parse_env_file(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let mut value = value.trim().to_string();

        if value.len() >= 2 {
            let first = value.chars().next().unwrap();
            let last = value.chars().last().unwrap();
            if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
                value = value[1..value.len() - 1].to_string();
            }
        }

        values.insert(key.to_string(), value);
    }

    values
}

pub fn env_file_in(dir: &Path) -> PathBuf {
    dir.join(ENV_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dot_env_file_is_parsed() {
        let values = parse_env_file(
            "# a comment\n\nCLIENT_ID=123.apps.googleusercontent.com\nCLIENT_SECRET=\"GOCSPX-secret\"\nEMPTY=\nNOEQUALS\n",
        );

        assert_eq!(
            values.get("CLIENT_ID").map(String::as_str),
            Some("123.apps.googleusercontent.com")
        );
        assert_eq!(values.get("CLIENT_SECRET").map(String::as_str), Some("GOCSPX-secret"));
        assert_eq!(values.get("EMPTY").map(String::as_str), Some(""));
        assert!(!values.contains_key("NOEQUALS"));
    }

    #[test]
    fn the_client_is_read_from_a_file() {
        let dir = std::env::temp_dir().join(format!("checkd-test-env-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        std::fs::write(
            dir.join(ENV_FILE),
            "CHECKD_GOOGLE_CLIENT_ID=123.apps.googleusercontent.com\nCHECKD_GOOGLE_CLIENT_SECRET=GOCSPX-secret\n",
        )
        .expect("write");

        if env_value(&CLIENT_ID_VARS).is_some() {
            return;
        }

        let resolved = resolve_client(&[dir.clone()])
            .expect("resolve")
            .expect("a client");
        assert_eq!(resolved.client.client_id, "123.apps.googleusercontent.com");
        assert_eq!(resolved.client.client_secret, "GOCSPX-secret");
        assert!(matches!(resolved.source, ClientSource::File(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_search_dirs_start_with_the_app_data_dir() {
        let dirs = search_dirs(Some(PathBuf::from("/tmp/checkd-app-data")));

        assert_eq!(dirs[0], PathBuf::from("/tmp/checkd-app-data"));
        assert!(dirs.len() >= 2);
    }
}
