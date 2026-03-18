use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyMode {
    System,
    None,
    Custom(String),
}

impl ProxyMode {
    pub fn to_db_string(&self) -> String {
        match self {
            ProxyMode::System => "system".into(),
            ProxyMode::None => "none".into(),
            ProxyMode::Custom(url) => format!("custom:{url}"),
        }
    }

    pub fn from_db_string(value: &str) -> Result<Self, String> {
        match value {
            "system" => Ok(ProxyMode::System),
            "none" => Ok(ProxyMode::None),
            s if s.starts_with("custom:") => {
                let url = &s["custom:".len()..];
                if url.is_empty() {
                    return Err("custom proxy mode requires a non-empty URL".into());
                }
                if !url.starts_with("http://")
                    && !url.starts_with("https://")
                    && !url.starts_with("socks5://")
                    && !url.starts_with("socks5h://")
                {
                    return Err(format!(
                        "invalid proxy URL scheme: expected http://, https://, socks5://, or socks5h://, got: {url}"
                    ));
                }
                Ok(ProxyMode::Custom(url.to_string()))
            }
            other => Err(format!(
                "invalid proxy mode: expected system, none, or custom:<url>, got: {other}"
            )),
        }
    }
}

impl Default for ProxyMode {
    fn default() -> Self {
        ProxyMode::System
    }
}

impl Serialize for ProxyMode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_db_string())
    }
}

impl<'de> Deserialize<'de> for ProxyMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ProxyMode::from_db_string(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub auto_switch_enabled: bool,
    pub cooldown_seconds: i64,
    pub refresh_interval_seconds: i64,
    pub network_query_concurrency: i64,
    pub proxy_mode: ProxyMode,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_switch_enabled: false,
            cooldown_seconds: 600,
            refresh_interval_seconds: 60,
            network_query_concurrency: 10,
            proxy_mode: ProxyMode::System,
        }
    }
}
