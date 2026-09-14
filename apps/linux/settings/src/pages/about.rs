//! 「关于」页：版本、许可证与随包数据的署名。
//!
//! 署名**必须在分发物里可见**——随包数据各有各的来源许可，有的要求署名，
//! 所以这一页不是装饰，是许可义务的落点（与 macOS 的「关于」页、Windows 的一致）。

use std::rc::Rc;

use crate::form;
use crate::settings::Settings;

pub fn build(settings: &Rc<Settings>) -> gtk::Box {
    let page = form::page();

    form::heading(&page, "青简输入法");
    form::note(&page, "输入的不只是文字。");
    form::note(
        &page,
        &format!("设置界面 {}　　qingjian.app", env!("CARGO_PKG_VERSION")),
    );
    form::note(&page, &format!("配置文件：{}", settings.path().display()));

    form::heading(&page, "许可");
    form::note(
        &page,
        "代码以 GPL-3.0-or-later 发布：可以自由使用、修改与再分发，修改后分发须同样开源。\
         「青简」名字与 logo 不在授权范围内。青简在官方渠道免费；若你为获得它向他人付费，你被骗了。",
    );

    form::heading(&page, "随包数据");
    form::note(
        &page,
        "词库源：规范字表、常用词表、THUOCL 领域词表。读音：Unihan 加 LLM 标注的多音字。\
         释义表：LLM 批量生成（词性 + 译词 + 日文假名）。emoji：Unicode CLDR 中文 annotations。\
         英文词表：ESDB / CSpell（MIT）。词汇等级：CEFR-J、Octanove、JLPT（经 Tanos / elzup）。\
         各自遵循来源的许可证，清单在仓库的 docs/design/landscape.md。",
    );

    form::heading(&page, "隐私");
    form::note(
        &page,
        "青简不上传任何数据。拼音转换、词库、学习、释义全部在本机完成，没有账号，没有统计上报。\
         云联想缺省关闭，打开后请求直接从你的电脑发到你自己填的服务商，不经过作者。",
    );
    page
}
