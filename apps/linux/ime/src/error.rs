//! 壳这一层的错误。Core 的错误各自有自己的类型，这里只包住「找不到数据 / 读不了配置」这类装配期问题。

use std::path::PathBuf;

/// Linux 壳的错误。
#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    /// 随包数据的根目录（其下有 `data/` 与 `assets/`）没找到：装机布局、开发布局、系统布局都不是。
    #[error(
        "bundled data root not found (looked next to the executable, in the repository, and in /usr/share/qingjian)"
    )]
    NoBundledRoot,

    /// 随包数据里少了某个文件。
    #[error("missing bundled resource: {0}")]
    MissingResource(PathBuf),

    /// 定位不到用户目录：`$HOME` 与 `$XDG_*` 都没有。
    #[error("cannot locate the user directory (neither $HOME nor $XDG_CONFIG_HOME is set)")]
    NoUserDirectory,

    /// 配置文件读不了或解析不了。
    #[error(transparent)]
    Config(#[from] qingjian_platform::ConfigError),

    /// 词库装不起来：这是唯一「装不上就不能用」的数据。
    #[error("load dictionary: {0}")]
    Dictionary(#[from] qingjian_dictionary::DictionaryError),

    /// 释义表装不起来。
    #[error("load glossary: {0}")]
    Glossary(#[from] qingjian_translate::GlossaryError),

    /// 语言模型装不起来。
    #[error("load language model: {0}")]
    LanguageModel(#[from] qingjian_lm::LmError),

    /// 连不上 IBus 总线，或者总线名被占了。
    #[error("ibus: {0}")]
    Ibus(String),
}
