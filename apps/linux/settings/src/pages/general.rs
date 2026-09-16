//! 「通用」页：学习语言、每页候选数、键盘方案、英文候选、标点。

use std::rc::Rc;

use crate::form::{self, Row};
use crate::settings::Settings;

/// 学习语言：只列打进包里有释义表的。
const LANGUAGES: &[(&str, &str)] = &[("en", "英语 English"), ("ja", "日语 日本語")];

/// 双拼方案；空串是全拼。
const SHUANGPIN: &[(&str, &str)] = &[
    ("", "全拼"),
    ("xiaohe", "小鹤双拼"),
    ("ziranma", "自然码"),
    ("microsoft", "微软双拼"),
    ("sogou", "搜狗双拼"),
];

/// 繁体输出：配置里的写法 + 界面名，与 `TraditionalVariant::ALL` 同序。
const TRADITIONAL: &[(&str, &str)] = &[
    ("off", "简体"),
    ("taiwan", "繁体（台湾正体）"),
    ("hongkong", "繁体（香港）"),
    ("standard", "繁体（通用字形）"),
];

pub fn build(settings: &Rc<Settings>) -> gtk::Box {
    let page = form::page();
    let general = &settings.config().general;

    form::heading(&page, "输入");
    form::choice(
        &page,
        settings,
        Row::new("学习语言", "general", "learning_language").hint("候选词旁显示哪种语言的译词"),
        LANGUAGES,
        &general.learning_language,
    );
    form::choice(
        &page,
        settings,
        Row::new("键盘方案", "general", "shuangpin").hint("双拼两键一个音节；选了之后全拼不再生效"),
        SHUANGPIN,
        &general.shuangpin,
    );
    form::switch(
        &page,
        settings,
        Row::new("大千注音", "general", "zhuyin").hint("按注音键盘输入，与双拼互斥"),
        general.zhuyin,
    );
    form::choice(
        &page,
        settings,
        Row::new("输出字形", "general", "traditional")
            .hint("台湾正体连用语一起换（软件 → 軟體）；词库与学习数据始终是简体"),
        TRADITIONAL,
        general.traditional.key(),
    );
    form::number(
        &page,
        settings,
        Row::new("每页候选数", "general", "page_size"),
        (1, 9),
        general.page_size as i64,
    );

    form::heading(&page, "英文与标点");
    form::switch(
        &page,
        settings,
        Row::new("英文模式给候选", "general", "english_candidates")
            .hint("关掉之后英文模式就是纯直通，不出补全与拼错纠正"),
        general.english_candidates,
    );
    form::switch(
        &page,
        settings,
        Row::new("中文模式用全角标点", "general", "full_width_punctuation"),
        general.full_width_punctuation,
    );
    form::switch(
        &page,
        settings,
        Row::new(
            "英文模式用全角标点",
            "general",
            "english_full_width_punctuation",
        ),
        general.english_full_width_punctuation,
    );
    form::note(
        &page,
        "「按应用分别设置」（[apps] 分节）在 Linux 上不生效：Wayland 不让客户端知道前台是哪个应用。",
    );
    page
}
