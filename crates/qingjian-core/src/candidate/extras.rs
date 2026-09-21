use serde::{Deserialize, Serialize};

/// 候选里出不出 emoji 与符号（配置项 `[general] extras`）。
///
/// 做成一个四选一而不是两个布尔：两个布尔在配置文件与设置界面里是两行、两处校验、
/// 四种组合都要想一遍，而用户心里只有「都要 / 只要一种 / 都不要」这一个维度。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtraCandidates {
    /// emoji 与符号都出，缺省。
    #[default]
    Both,

    /// 只出 emoji（笑 → 😄）。
    Emoji,

    /// 只出符号（`duigou` → ✔）。
    Symbol,

    /// 都不出，候选里只有文字。
    Off,
}

impl ExtraCandidates {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 4] = [Self::Both, Self::Emoji, Self::Symbol, Self::Off];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Emoji => "emoji",
            Self::Symbol => "symbol",
            Self::Off => "off",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Both => "emoji 与符号",
            Self::Emoji => "只要 emoji",
            Self::Symbol => "只要符号",
            Self::Off => "都不要",
        }
    }

    /// 出不出 emoji。
    pub fn emoji(self) -> bool {
        matches!(self, Self::Both | Self::Emoji)
    }

    /// 出不出符号。
    pub fn symbol(self) -> bool {
        matches!(self, Self::Both | Self::Symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 配置里的写法与枚举一一对上，设置界面按 `key` 存、按 `ALL` 列。
    #[test]
    fn keys_round_trip_through_toml() {
        for value in ExtraCandidates::ALL {
            #[derive(serde::Deserialize)]
            struct Holder {
                value: ExtraCandidates,
            }
            let holder: Holder =
                toml::from_str(&format!("value = \"{}\"", value.key())).expect("解析");
            assert_eq!(holder.value, value);
        }
    }

    /// 缺省两样都出：没写这一项的老配置行为不变。
    #[test]
    fn default_shows_both() {
        let default = ExtraCandidates::default();
        assert!(default.emoji() && default.symbol());
    }

    #[test]
    fn each_value_selects_what_it_says() {
        assert!(ExtraCandidates::Emoji.emoji() && !ExtraCandidates::Emoji.symbol());
        assert!(!ExtraCandidates::Symbol.emoji() && ExtraCandidates::Symbol.symbol());
        assert!(!ExtraCandidates::Off.emoji() && !ExtraCandidates::Off.symbol());
    }
}
