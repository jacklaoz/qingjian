//! 数据文件位置。两头各一套规矩：
//!
//! - **只读的随包数据**（词库、语言模型、释义表、emoji、等级表、模型）走 [`qingjian_platform::resources`]，
//!   它按「与可执行文件同级 → 仓库开发布局 → `/usr/share/qingjian`」找。
//! - **用户数据**按 XDG 基础目录规范放三处：配置 `$XDG_CONFIG_HOME/qingjian`、
//!   数据 `$XDG_DATA_HOME/qingjian`、日志 `$XDG_STATE_HOME/qingjian`。
//!   macOS 壳把这三样全塞在 `~/Library/Application Support/Qingjian/`，Linux 上分开才是本地惯例。

use std::path::PathBuf;

use qingjian_platform::resources;

use crate::error::ShellError;

/// 三个 XDG 目录下共用的产品目录名。
const APP_DIR: &str = "qingjian";

/// 配置目录 `$XDG_CONFIG_HOME/qingjian`（缺省 `~/.config/qingjian`），不存在则创建。
pub fn config_dir() -> Result<PathBuf, ShellError> {
    ensure(xdg_dir("XDG_CONFIG_HOME", ".config")?)
}

/// 用户数据目录 `$XDG_DATA_HOME/qingjian`（缺省 `~/.local/share/qingjian`），不存在则创建。
/// 学习数据六张表、输入日志、统计、词汇记录、个人释义表都落在这里。
pub fn data_dir() -> Result<PathBuf, ShellError> {
    ensure(xdg_dir("XDG_DATA_HOME", ".local/share")?)
}

/// 日志目录 `$XDG_STATE_HOME/qingjian`（缺省 `~/.local/state/qingjian`），不存在则创建。
/// 日志是「重启后可以丢、但留着有用」的状态，XDG 规定放 state 而不是 data。
pub fn state_dir() -> Result<PathBuf, ShellError> {
    ensure(xdg_dir("XDG_STATE_HOME", ".local/state")?)
}

/// 配置文件 `$XDG_CONFIG_HOME/qingjian/config.toml`。
pub fn config_file() -> Result<PathBuf, ShellError> {
    Ok(config_dir()?.join("config.toml"))
}

/// 用户导入的词库目录 `$XDG_DATA_HOME/qingjian/dicts/`，不存在则创建。
pub fn dicts_dir() -> Result<PathBuf, ShellError> {
    ensure(data_dir()?.join("dicts"))
}

/// 随包资源的完整路径（相对随包根，如 `data/generated/dict.qj`）；找不到根或文件不在时报错，
/// 而不是等到读取时才炸。
pub fn bundled(rel: &str) -> Result<PathBuf, ShellError> {
    let root = resources::bundled_root().ok_or(ShellError::NoBundledRoot)?;
    let path = root.join(rel);
    if path.exists() {
        Ok(path)
    } else {
        Err(ShellError::MissingResource(path))
    }
}

/// 随包资源的完整路径，缺了就是 `None`（给那些「没有就少个功能」的数据用：语言模型、释义表、emoji）。
pub fn optional_bundled(rel: &str) -> Option<PathBuf> {
    bundled(rel).ok()
}

/// 一个 XDG 目录：环境变量有且是绝对路径就用它，否则 `$HOME` 接上缺省相对路径。
fn xdg_dir(variable: &str, fallback: &str) -> Result<PathBuf, ShellError> {
    // XDG 规定相对路径的变量值要当作没设，不然会在当前工作目录下乱建目录
    if let Some(value) = std::env::var_os(variable) {
        let base = PathBuf::from(value);
        if base.is_absolute() {
            return Ok(base.join(APP_DIR));
        }
    }
    let home = std::env::var_os("HOME").ok_or(ShellError::NoUserDirectory)?;
    Ok(PathBuf::from(home).join(fallback).join(APP_DIR))
}

/// 建目录；建不了当成定位不到用户目录，调用方据此退回只在内存里学习。
fn ensure(dir: PathBuf) -> Result<PathBuf, ShellError> {
    std::fs::create_dir_all(&dir).map_err(|_| ShellError::NoUserDirectory)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 环境变量是绝对路径时直接用它，产品目录接在后面。
    #[test]
    fn absolute_xdg_variable_wins() {
        unsafe { std::env::set_var("QINGJIAN_TEST_XDG", "/var/tmp/xdg") };
        let dir = xdg_dir("QINGJIAN_TEST_XDG", ".config").unwrap();
        assert_eq!(dir, PathBuf::from("/var/tmp/xdg/qingjian"));
        unsafe { std::env::remove_var("QINGJIAN_TEST_XDG") };
    }

    /// 相对路径按 XDG 规定当作没设，退回 `$HOME` 加缺省相对路径，免得在工作目录里乱建。
    #[test]
    fn relative_xdg_variable_falls_back_to_home() {
        unsafe { std::env::set_var("QINGJIAN_TEST_XDG_REL", "relative/path") };
        let dir = xdg_dir("QINGJIAN_TEST_XDG_REL", ".config").unwrap();
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        assert_eq!(dir, home.join(".config/qingjian"));
        unsafe { std::env::remove_var("QINGJIAN_TEST_XDG_REL") };
    }
}
