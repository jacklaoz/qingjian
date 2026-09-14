//! XDG 基础目录（Linux）。
//!
//! 输入法进程与设置界面是两个可执行文件，都要定位同一批目录，所以放在共用的地方而不是各抄一份。
//! macOS 把配置、数据、日志全塞在 `~/Library/Application Support/Qingjian/`，Linux 上分三处才是本地惯例。

use std::path::PathBuf;

/// 三个 XDG 目录下共用的产品目录名。
const APP_DIR: &str = "qingjian";

/// 配置目录 `$XDG_CONFIG_HOME/qingjian`（缺省 `~/.config/qingjian`）。不建目录。
pub fn config_dir() -> Option<PathBuf> {
    dir("XDG_CONFIG_HOME", ".config")
}

/// 用户数据目录 `$XDG_DATA_HOME/qingjian`（缺省 `~/.local/share/qingjian`）：
/// 学习数据六张表、输入日志、统计、词汇记录、个人释义表、导入的词库。不建目录。
pub fn data_dir() -> Option<PathBuf> {
    dir("XDG_DATA_HOME", ".local/share")
}

/// 日志目录 `$XDG_STATE_HOME/qingjian`（缺省 `~/.local/state/qingjian`）。
/// 日志是「重启后可以丢、但留着有用」的状态，XDG 规定放 state 而不是 data。不建目录。
pub fn state_dir() -> Option<PathBuf> {
    dir("XDG_STATE_HOME", ".local/state")
}

/// 配置文件 `$XDG_CONFIG_HOME/qingjian/config.toml`。
pub fn config_file() -> Option<PathBuf> {
    Some(config_dir()?.join("config.toml"))
}

/// 一个 XDG 目录：环境变量有且是绝对路径就用它，否则 `$HOME` 接上缺省相对路径。
///
/// XDG 规定相对路径的变量值要当作没设，不然会在当前工作目录下乱建目录。
fn dir(variable: &str, fallback: &str) -> Option<PathBuf> {
    if let Some(value) = std::env::var_os(variable) {
        let base = PathBuf::from(value);
        if base.is_absolute() {
            return Some(base.join(APP_DIR));
        }
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(fallback).join(APP_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 环境变量是绝对路径时直接用它，产品目录接在后面。
    #[test]
    fn absolute_variable_wins() {
        unsafe { std::env::set_var("QINGJIAN_TEST_XDG_ABS", "/var/tmp/xdg") };
        assert_eq!(
            dir("QINGJIAN_TEST_XDG_ABS", ".config"),
            Some(PathBuf::from("/var/tmp/xdg/qingjian"))
        );
        unsafe { std::env::remove_var("QINGJIAN_TEST_XDG_ABS") };
    }

    /// 相对路径按 XDG 规定当作没设，退回 `$HOME`，免得在工作目录里乱建。
    #[test]
    fn relative_variable_falls_back_to_home() {
        unsafe { std::env::set_var("QINGJIAN_TEST_XDG_REL", "relative/path") };
        let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"));
        assert_eq!(
            dir("QINGJIAN_TEST_XDG_REL", ".config"),
            Some(home.join(".config/qingjian"))
        );
        unsafe { std::env::remove_var("QINGJIAN_TEST_XDG_REL") };
    }

    /// 三个目录各走各的变量，别搞混。
    #[test]
    fn the_three_directories_use_their_own_variables() {
        let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"));
        for variable in ["XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_STATE_HOME"] {
            unsafe { std::env::remove_var(variable) };
        }
        assert_eq!(config_dir(), Some(home.join(".config/qingjian")));
        assert_eq!(data_dir(), Some(home.join(".local/share/qingjian")));
        assert_eq!(state_dir(), Some(home.join(".local/state/qingjian")));
    }
}
