//! 启动时的装配：读配置、定位随包数据、装 Engine、建 Router。
//!
//! 与 Windows Server 的 `main.rs`、macOS 的 `host/init.rs` 是同一件事，路径来源不同而已。

use std::path::{Path, PathBuf};

use qingjian_core::{Engine, Language};
use qingjian_platform::Config;

use crate::assembly::{self, AssemblySpec, LanguageModelFiles};
use crate::dispatch::{Router, RouterConfig};
use crate::error::ShellError;
use crate::paths;

/// 读配置、装 Engine、建 Router。配置文件不存在时先写一份带注释的模板。
pub fn build() -> Result<(Router, PathBuf), ShellError> {
    let config_path = paths::config_file()?;
    if let Err(error) = Config::write_template_if_missing(&config_path) {
        tracing::warn!(%error, "写配置模板失败");
    }
    let config = Config::load(&config_path).unwrap_or_else(|error| {
        tracing::error!(%error, "配置读不了，用缺省值");
        Config::default()
    });
    let engine = assemble(&config)?;
    let router = Router::new(engine, RouterConfig::from(&config));
    Ok((router, config_path))
}

/// 按配置与随包数据装一个 Engine。词库装不起来就回落样例词库，样例也不行才报错。
fn assemble(config: &Config) -> Result<Engine, ShellError> {
    let root = qingjian_platform::resources::bundled_root().unwrap_or_else(|| PathBuf::from("."));
    let language = learning_language(config);
    let user_dir = paths::data_dir().ok();
    let spec = AssemblySpec {
        glossary: glossary_file(&root, language).map(|path| (language, path)),
        english_glossary: glossary_file(&root, Language::Chinese),
        english: generated(&root, "english.tsv"),
        emoji: ["emoji-zh.tsv", "emoji-en.tsv"]
            .into_iter()
            .filter_map(|name| asset(&root, &format!("emoji/{name}")))
            .collect(),
        language_model: LanguageModelFiles::find(&root.join("data/generated")),
        bundled_dicts_dir: Some(root.join("data/generated/dicts")).filter(|dir| dir.is_dir()),
        dictionaries: config.dictionaries.clone(),
        levels_dir: Some(root.join("assets/levels")),
        user_dir,
        input_log: config.general.input_log,
        ..AssemblySpec::new(default_dict(&root))
    };
    let mut spec = spec;
    match assembly::assemble(&spec) {
        Ok(engine) => Ok(engine),
        Err(error) => {
            tracing::error!(%error, dict = %spec.dict.display(), "正式词库装配失败，回落样例词库");
            spec.dict = sample_dict(&root);
            assembly::assemble(&spec)
        }
    }
}

/// 配置里的学习语言；不认识的按英文。
fn learning_language(config: &Config) -> Language {
    let code = &config.general.learning_language;
    code.parse().unwrap_or_else(|_| {
        tracing::warn!(code, "不认识的学习语言，按英文");
        Language::English
    })
}

/// `<root>/data/generated/<name>`，不存在为 `None`。
fn generated(root: &Path, name: &str) -> Option<PathBuf> {
    existing(root.join("data/generated").join(name))
}

/// `<root>/assets/<rel>`，不存在为 `None`。
fn asset(root: &Path, rel: &str) -> Option<PathBuf> {
    existing(root.join("assets").join(rel))
}

fn existing(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// 正式词库，没有就回落手写样例。
fn default_dict(root: &Path) -> PathBuf {
    generated(root, "dict.qj").unwrap_or_else(|| sample_dict(root))
}

fn sample_dict(root: &Path) -> PathBuf {
    root.join("assets/sample/dict.tsv")
}

/// 某语言的释义表：打包过的优先，否则随 git 的 TSV。
fn glossary_file(root: &Path, language: Language) -> Option<PathBuf> {
    let code = language.code();
    generated(root, &format!("glossary-{code}.qj"))
        .or_else(|| asset(root, &format!("glossary/glossary-{code}.tsv")))
}
