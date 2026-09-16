//! Router 要用的配置项。

use qingjian_core::{ShuangpinScheme, TraditionalVariant};
use qingjian_platform::protocol::KeyModifiers;
use qingjian_platform::{Config, LayoutMode, ThemeMode};

/// Router 要用的配置项，与 macOS 壳的 `Host` 字段、Windows 的 `RouterConfig` 对齐。
///
/// **没有 `apps` 字段**：`[apps]` 按应用配置在 Linux 上拿不到前台应用标识，永远不命中，
/// 留个读了不用的字段只会让人以为它生效（见 `docs/plan/linux_plan.md`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterConfig {
    /// 每页候选数（`[general] page_size`）。
    pub page_size: usize,

    /// 云端候选在第一页预留的格数（`[predict] slots`）。
    pub cloud_slots: usize,

    /// 候选排布（`[general] layout`）。方案 C 交给 IBus 面板时用不上，方案 A 自绘时才生效。
    pub layout: LayoutMode,

    /// 候选窗口外观（`[general] theme`）。同上。
    pub theme: ThemeMode,

    /// 翻页键对（`[general] page_keys`，上一页 / 下一页）。
    pub page_keys: (char, char),

    /// 英文模式给不给英文候选（`[general] english_candidates`）。
    pub english_candidates: bool,

    /// 中文模式下不在组句时的标点转全角（`[general] full_width_punctuation`）。
    pub full_width: bool,

    /// 英文模式的那一份（`[general] english_full_width_punctuation`）。
    pub english_full_width: bool,

    /// 大千注音（`[general] zhuyin`）。
    pub zhuyin: bool,

    /// 上屏第一 / 第二个译词的修饰键（`[shortcut] translation` / `translation_second`）。
    pub translation_keys: (KeyModifiers, KeyModifiers),

    /// 删候选的修饰键（`[shortcut] delete_candidate`）。
    pub delete_keys: KeyModifiers,

    /// 双拼方案（`[general] shuangpin`）；全拼为 `None`。
    pub shuangpin: Option<ShuangpinScheme>,

    /// 繁体输出（`[general] traditional`）；缺省不转。
    pub traditional: TraditionalVariant,
}

impl From<&Config> for RouterConfig {
    fn from(config: &Config) -> Self {
        Self {
            page_size: config.general.page_size(),
            cloud_slots: config.predict.slots,
            layout: config.general.layout,
            theme: config.general.theme,
            page_keys: config.general.page_keys(),
            english_candidates: config.general.english_candidates,
            full_width: config.general.full_width_punctuation,
            english_full_width: config.general.english_full_width_punctuation,
            zhuyin: config.general.zhuyin,
            translation_keys: {
                let (first, second) = config.shortcut.translation_keys();
                (first.into(), second.into())
            },
            delete_keys: config.shortcut.delete_keys().into(),
            shuangpin: config.general.shuangpin(),
            traditional: config.general.traditional,
        }
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}
