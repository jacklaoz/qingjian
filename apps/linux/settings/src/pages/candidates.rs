//! 「候选窗口」页：翻页键与拼音行。
//!
//! 外观（浅色 / 深色）与排布（竖排 / 横排）这两项**在 Linux 上还不生效**：候选窗现在由 IBus 面板画，
//! 样式跟着面板主题走。等自绘那一步做出来（只有 wlroots 系拿得到，见 docs/plan/linux_plan.md）才接上，
//! 所以这里只说明、不摆控件——摆一个改了没反应的开关比不摆更糟。

use std::rc::Rc;

use crate::form::{self, Row};
use crate::settings::Settings;

/// 翻页键对。
const PAGE_KEYS: &[(&str, &str)] = &[("[]", "[ 和 ]"), (",.", "逗号和句号")];

pub fn build(settings: &Rc<Settings>) -> gtk::Box {
    let page = form::page();
    let general = &settings.config().general;

    form::heading(&page, "翻页");
    form::choice(
        &page,
        settings,
        Row::new("翻页键", "general", "page_keys")
            .hint("选逗号句号的话，组句时敲它们是翻页而不是上屏加标点"),
        PAGE_KEYS,
        &general.page_keys,
    );

    form::heading(&page, "外观");
    form::note(
        &page,
        "候选窗口现在由系统的输入法面板绘制，浅色 / 深色与竖排 / 横排跟着面板走，改这里不生效。\
         等青简自绘候选窗之后再接上（只有 sway / Hyprland 这类合成器拿得到所需的 Wayland 协议）。",
    );
    form::note(&page, "面板的样式在系统的「IBus 首选项」里调：ibus-setup。");
    page
}
