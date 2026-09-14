//! 各分节页：一个文件一页，每页一个 `build(settings) -> gtk::Box`。
//! 页的划分与 macOS 偏好设置、Windows 设置对齐，方便三端对照着改。

mod about;
mod advanced;
mod candidates;
mod cloud;
mod fuzzy;
mod general;
mod shortcut;

use std::rc::Rc;

use crate::settings::Settings;

/// 侧栏里的一页：`(标识, 标题, 内容)`。
pub fn all(settings: &Rc<Settings>) -> Vec<(&'static str, &'static str, gtk::Box)> {
    vec![
        ("general", "通用", general::build(settings)),
        ("candidates", "候选窗口", candidates::build(settings)),
        ("shortcut", "快捷键", shortcut::build(settings)),
        ("fuzzy", "模糊音", fuzzy::build(settings)),
        ("cloud", "云服务", cloud::build(settings)),
        ("advanced", "高级", advanced::build(settings)),
        ("about", "关于", about::build(settings)),
    ]
}
