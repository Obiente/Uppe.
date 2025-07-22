use std::{env, fmt, fs, path};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    ReadFailed(()),
    WriteFailed(()),
    ParseFailed(()),
    ConfigPathUnavailable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub zeromq: ZeroMQ,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZeroMQ {
    pub bind: String,
    pub port: u16,
}

/// Used to ensure we are actually reading a toml file
fn normalize_toml_path(path: &path::Path) -> path::PathBuf {
    let mut path = path.to_path_buf();
    if path.extension().map(|ext| ext != "toml").unwrap_or(true) {
        path.set_extension("toml");
    }
    path
}

/// Get default config path ($XDG_CONFIG_HOME/uppe/config.toml or
/// $HOME/.config/...)
fn default_config_path() -> Result<path::PathBuf, Error> {
    let path = if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        path::PathBuf::from(config_home)
    } else if let Some(home_dir) = env::home_dir() {
        home_dir.join(".config")
    } else {
        return Err(Error::ConfigPathUnavailable);
    };

    Ok(path.join("uppe/config.toml"))
}

impl Default for Config {
    fn default() -> Self {
        Self { zeromq: ZeroMQ { bind: "*".into(), port: 5555 } }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let write_indented = |level: usize| {
            move |f: &mut fmt::Formatter<'_>, label: &str, value: &dyn fmt::Display| {
                writeln!(f, "  {:indent$}{}: {}", "", label, value, indent = level * 2)
            }
        };
        let write_title_indented = |level: usize| {
            move |f: &mut fmt::Formatter<'_>, label: &str| {
                writeln!(f, "{:indent$}{}", "", label, indent = level * 2)
            }
        };

        let write_title_1 = write_title_indented(1);
        let write_1 = write_indented(1);

        writeln!(f, "Current Internal Configuration State:")?;
        write_title_1(f, "ZeroMQ")?;
        write_1(f, "Bind Address", &self.zeromq.bind)?;
        write_1(f, "Port", &self.zeromq.port)?;

        Ok(())
    }
}

impl Config {
    /// Generate Config structure from file
    ///
    /// Creates a default config in ~/.config/uppe/config.toml
    ///  or the specified path, with the name config.toml if one does not exist
    ///
    /// ```rust
    /// let cfg = config::Config::from_config(None::<&path::Path>)?;
    /// println!("{}", cfg);
    /// ```
    pub fn from_config(optional_path: Option<impl AsRef<path::Path>>) -> Result<Self, Error> {
        let config_path: path::PathBuf = if let Some(path) = optional_path {
            normalize_toml_path(path.as_ref())
        } else {
            default_config_path()?
        };

        if config_path.exists() {
            let raw_string =
                fs::read_to_string(&config_path).map_err(|_err| Error::ReadFailed(()))?;
            toml::from_str(raw_string.as_str()).map_err(|_err| Error::ParseFailed(()))
        } else {
            let config = Self::default();
            config.write_config(&config_path)?;
            Ok(config)
        }
    }

    /// Serialize and write a config to a file
    pub fn write_config(&self, path: &std::path::Path) -> Result<(), Error> {
        let config_str: String =
            toml::to_string_pretty(self).map_err(|_err| Error::ParseFailed(()))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_err| Error::WriteFailed(()))?;
        }

        std::fs::write(path, config_str).map_err(|_err| Error::WriteFailed(()))
    }
}
