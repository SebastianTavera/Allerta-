//! Configuración desde `faro.env` (formato CLAVE=valor) y variables de
//! entorno `FARO_*`, que tienen prioridad sobre el archivo.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_RECIPIENT: &str = "joanromana.st@academia.umb.edu.co";
const CONFIG_FILE: &str = "faro.env";
const DEFAULT_SMTP_PORT: u16 = 587;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub database_path: PathBuf,
    pub recipient: String,
    pub smtp: Option<SmtpConfig>,
}

impl AppConfig {
    pub fn load() -> Self {
        let mut vars = config_file_vars();
        vars.extend(env::vars().filter(|(key, _)| key.starts_with("FARO_")));
        Self::from_vars(&vars)
    }

    pub fn from_vars(vars: &HashMap<String, String>) -> Self {
        let get = |key: &str| vars.get(key).map(|value| value.trim()).filter(|value| !value.is_empty());

        let smtp = match (get("FARO_SMTP_HOST"), get("FARO_SMTP_USER"), get("FARO_SMTP_PASSWORD")) {
            (Some(host), Some(username), Some(password)) => Some(SmtpConfig {
                host: host.to_owned(),
                port: get("FARO_SMTP_PORT")
                    .and_then(|port| port.parse().ok())
                    .unwrap_or(DEFAULT_SMTP_PORT),
                username: username.to_owned(),
                password: password.to_owned(),
                from: get("FARO_SMTP_FROM").unwrap_or(username).to_owned(),
            }),
            _ => None,
        };

        Self {
            database_path: get("FARO_DB_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| data_dir().join("faro.db")),
            recipient: get("FARO_NOTIFY_TO").unwrap_or(DEFAULT_RECIPIENT).to_owned(),
            smtp,
        }
    }
}

fn config_file_vars() -> HashMap<String, String> {
    candidate_paths()
        .into_iter()
        .find_map(|path| fs::read_to_string(path).ok())
        .map(|content| parse_env(&content))
        .unwrap_or_default()
}

/// Busca `faro.env` en: carpeta actual, carpeta del ejecutable, carpeta de datos.
fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(CONFIG_FILE)];
    if let Some(dir) = env::current_exe().ok().and_then(|exe| exe.parent().map(PathBuf::from)) {
        paths.push(dir.join(CONFIG_FILE));
    }
    paths.push(data_dir().join(CONFIG_FILE));
    paths
}

pub fn parse_env(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_owned(), unquote(value.trim()).to_owned()))
        })
        .collect()
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// Carpeta de datos por usuario según el sistema operativo.
fn data_dir() -> PathBuf {
    let base = if cfg!(windows) {
        env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
    };
    base.map(|dir| dir.join("faro")).unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comments_quotes_and_export() {
        let vars = parse_env(
            "# comentario\nFARO_SMTP_HOST = smtp.gmail.com\nexport FARO_SMTP_PASSWORD=\"abcd efgh\"\n\nBASURA\n",
        );
        assert_eq!(vars["FARO_SMTP_HOST"], "smtp.gmail.com");
        assert_eq!(vars["FARO_SMTP_PASSWORD"], "abcd efgh");
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn smtp_requires_host_user_and_password() {
        let mut vars = HashMap::new();
        vars.insert("FARO_SMTP_HOST".to_owned(), "smtp.gmail.com".to_owned());
        vars.insert("FARO_SMTP_USER".to_owned(), "faro@gmail.com".to_owned());
        let config = AppConfig::from_vars(&vars);
        assert!(config.smtp.is_none());
        assert_eq!(config.recipient, DEFAULT_RECIPIENT);

        vars.insert("FARO_SMTP_PASSWORD".to_owned(), "secreto".to_owned());
        let smtp = AppConfig::from_vars(&vars).smtp.unwrap();
        assert_eq!(smtp.port, 587);
        assert_eq!(smtp.from, "faro@gmail.com");
    }
}
