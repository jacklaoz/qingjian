//! 「云服务」页：本地整句模型开关，以及云联想的接口与密钥。
//!
//! 云联想**缺省关闭**。打开之后请求直接从本机发到用户自己填的服务商，不经作者；
//! 发出去的只有当前这段拼音与（可选的）光标附近文本。

use std::rc::Rc;

use crate::form::{self, Row};
use crate::settings::Settings;

/// 思考强度：DeepSeek V4 这类默认思考的模型不关会把 token 预算花在思考上、正文为空。
const EFFORT: &[(&str, &str)] = &[
    ("none", "不思考（缺省）"),
    ("low", "低"),
    ("medium", "中"),
    ("high", "高"),
];

pub fn build(settings: &Rc<Settings>) -> gtk::Box {
    let page = form::page();
    let config = settings.config();

    form::heading(&page, "本地整句模型");
    form::switch(
        &page,
        settings,
        Row::new("停顿后重排整句候选", "model", "enabled")
            .hint("字级 Transformer，全程离线；不装 qingjian-model 这个包就没有模型可加载"),
        config.model.enabled,
    );

    form::heading(&page, "云联想");
    form::note(
        &page,
        "缺省关闭。打开后请求直接从你的电脑发到你自己填的服务商，不经过作者；密码框里绝不发送。",
    );
    form::switch(
        &page,
        settings,
        Row::new("打开云联想", "predict", "enabled"),
        config.predict.enabled,
    );
    form::text(
        &page,
        settings,
        Row::new("接口地址", "predict", "base_url")
            .hint("OpenAI 兼容接口，如 https://api.deepseek.com"),
        &config.predict.base_url,
    );
    form::text(
        &page,
        settings,
        Row::new("模型", "predict", "model"),
        &config.predict.model,
    );
    form::text(
        &page,
        settings,
        Row::new("密钥环境变量", "predict", "api_key_env")
            .hint("密钥从这个环境变量读；名字跟产品不跟供应商，因为接口地址本来就能指到别家"),
        &config.predict.api_key_env,
    );
    form::choice(
        &page,
        settings,
        Row::new("思考强度", "predict", "reasoning_effort"),
        EFFORT,
        &config.predict.reasoning_effort,
    );
    form::number(
        &page,
        settings,
        Row::new("云端候选占几格", "predict", "slots")
            .hint("云端词补进第一页末尾这么多格，前面的本地候选一格不挪"),
        (0, 4),
        config.predict.slots as i64,
    );
    page
}
