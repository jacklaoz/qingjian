//! 数据文件位置。两头各一套规矩：
//!
//! - **只读的随包数据**（词库、语言模型、释义表、emoji、等级表、模型）走 [`qingjian_platform::resources`]，
//!   它按「与可执行文件同级 → 仓库开发布局 → `/usr/share/qingjian`」找。
//! - **用户数据**按 XDG 基础目录规范放三处，定位在 [`qingjian_platform::xdg`]（设置界面是另一个可执行文件，共用它）；
//!   这里只多做一件事：不存在就建出来。

use std::path::PathBuf;

use qingjian_platform::{resources, xdg};

use crate::error::ShellError;

/// 配置目录，不存在则创建。
pub fn config_dir() -> Result<PathBuf, ShellError> {
    ensure(xdg::config_dir())
}

/// 用户数据目录，不存在则创建。
pub fn data_dir() -> Result<PathBuf, ShellError> {
    ensure(xdg::data_dir())
}

/// 日志目录，不存在则创建。
pub fn state_dir() -> Result<PathBuf, ShellError> {
    ensure(xdg::state_dir())
}

/// 配置文件 `config.toml`。
pub fn config_file() -> Result<PathBuf, ShellError> {
    Ok(config_dir()?.join("config.toml"))
}

/// 用户导入的词库目录 `dicts/`，不存在则创建。
pub fn dicts_dir() -> Result<PathBuf, ShellError> {
    ensure(data_dir().ok().map(|dir| dir.join("dicts")))
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

/// 建目录；定位不到或建不了都当成没有用户目录，调用方据此退回只在内存里学习。
fn ensure(dir: Option<PathBuf>) -> Result<PathBuf, ShellError> {
    let dir = dir.ok_or(ShellError::NoUserDirectory)?;
    std::fs::create_dir_all(&dir).map_err(|_| ShellError::NoUserDirectory)?;
    Ok(dir)
}
