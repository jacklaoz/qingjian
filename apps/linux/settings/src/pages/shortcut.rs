//! 「快捷键」页：上屏译词、删候选的修饰键，以及表达式 / 问字的前缀键。
//!
//! 修饰键按 macOS 命名写（option / control / command），落到 Linux 是 Alt / Ctrl / Super，
//! 三个平台共用一份配置（见 `qingjian_platform::config::Modifiers`）。

use std::rc::Rc;

use crate::form::{self, Row};
use crate::settings::Settings;

pub fn build(settings: &Rc<Settings>) -> gtk::Box {
    let page = form::page();
    let shortcut = &settings.config().shortcut;

    form::heading(&page, "修饰键 + 数字");
    form::note(
        &page,
        "只在组句时才拦；没在打字时这些组合照常交给应用。\
         写法：option / shift / control / command 用 + 连（在 Linux 上 option 是 Alt、command 是 Super）。",
    );
    form::text(
        &page,
        settings,
        Row::new("上屏第一个译词", "shortcut", "translation")
            .hint("不少窗口管理器把 Alt + 数字占去切工作区了，被占了就改成 control+option"),
        &shortcut.translation.to_string(),
    );
    form::text(
        &page,
        settings,
        Row::new("上屏第二个译词", "shortcut", "translation_second").hint("候选右侧有两个译词时"),
        &shortcut.translation_second.to_string(),
    );
    form::text(
        &page,
        settings,
        Row::new("删掉候选", "shortcut", "delete_candidate")
            .hint("用户词整个删掉，词库词清掉对它的学习记录"),
        &shortcut.delete_candidate.to_string(),
    );

    form::heading(&page, "前缀键");
    form::note(
        &page,
        "表达式（v1+2 算出 3）与问字（?zi 问云端）的起始键，只能是 v / u / i。",
    );
    page
}
