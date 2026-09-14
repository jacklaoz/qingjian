//! 「高级」页：日志、输入日志，以及打开各个目录。

use std::rc::Rc;

use qingjian_platform::xdg;

use crate::form::{self, Row};
use crate::settings::Settings;

/// 日志级别。缺省 info 不含敲的内容；debug 会逐键记，排查完记得关。
const LEVELS: &[(&str, &str)] = &[
    ("error", "只记错误"),
    ("warn", "警告"),
    ("info", "常规（缺省）"),
    ("debug", "详细，逐键记"),
];

pub fn build(settings: &Rc<Settings>) -> gtk::Box {
    let page = form::page();
    let general = &settings.config().general;

    form::heading(&page, "日志");
    form::choice(
        &page,
        settings,
        Row::new("日志级别", "general", "log_level")
            .hint("缺省不记你敲了什么；「详细」会逐键记，排查完记得调回去"),
        LEVELS,
        general.log_level.key(),
    );
    form::switch(
        &page,
        settings,
        Row::new("输入日志", "general", "input_log")
            .hint("每次上屏记一行（敲了什么、选了第几个），只写在本机，用来离线评测排序效果"),
        general.input_log,
    );

    form::heading(&page, "目录");
    open_button(&page, "打开配置文件所在目录", xdg::config_dir());
    open_button(&page, "打开数据目录（学习数据、输入日志）", xdg::data_dir());
    open_button(&page, "打开日志目录", xdg::state_dir());
    form::note(
        &page,
        "配置改完即时生效：输入法每秒看一次配置文件，变了就热加载，不用重启。",
    );
    page
}

/// 用桌面的默认文件管理器打开一个目录；没有 `xdg-open` 时只记日志，不弹框打断用户。
fn open_button(page: &gtk::Box, title: &str, dir: Option<std::path::PathBuf>) {
    let Some(dir) = dir else {
        return;
    };
    form::button(page, title, move || {
        let _ = std::fs::create_dir_all(&dir);
        match std::process::Command::new("xdg-open").arg(&dir).spawn() {
            Ok(_) => tracing::debug!(dir = %dir.display(), "打开目录"),
            Err(error) => tracing::warn!(%error, dir = %dir.display(), "打不开目录"),
        }
    });
}
