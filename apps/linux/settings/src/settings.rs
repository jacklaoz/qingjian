//! 配置的读与写。
//!
//! **写回走 `Config::set_value`（toml_edit 原地改键）而不是整份序列化**：配置模板里每一项都有注释，
//! 整份写回会把注释冲掉。这与 macOS 偏好设置、Windows 设置是同一条规矩。
//!
//! 界面不需要把改动推给输入法：输入法进程每秒看一次配置文件的 mtime，变了就热加载（`ime/src/run.rs`）。

use std::path::PathBuf;

use qingjian_platform::{Config, ConfigError, xdg};

/// 当前配置与它的文件位置。
pub struct Settings {
    /// `~/.config/qingjian/config.toml`。
    path: PathBuf,

    /// 启动时读到的那一份，用来给控件填初值。
    config: Config,
}

impl Settings {
    /// 读配置；文件不存在先写一份带注释的模板，读不了就用缺省值（界面照常能开，改了照样写得进去）。
    pub fn load() -> Self {
        let path = xdg::config_file().unwrap_or_else(|| PathBuf::from("config.toml"));
        if let Err(error) = Config::write_template_if_missing(&path) {
            tracing::warn!(%error, "写配置模板失败");
        }
        let config = Config::load(&path).unwrap_or_else(|error| {
            tracing::error!(%error, "配置读不了，界面按缺省值显示");
            Config::default()
        });
        Self { path, config }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// 改一个键并落盘。失败只记日志：设置界面不该因为一次写失败就弹框打断用户。
    pub fn set(&self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        match Config::set_value(&self.path, section, key, value) {
            Ok(()) => tracing::debug!(section, key, "配置已写回"),
            Err(error) => report(&error),
        }
    }
}

fn report(error: &ConfigError) {
    tracing::error!(%error, "写配置失败");
}
