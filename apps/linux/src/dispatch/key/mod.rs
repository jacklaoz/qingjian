//! 按键处理：分流在 [`input`]，「修饰键 + 数字」快捷键在 [`shortcut`]，一次按键的结果是 [`Effect`]。

mod effect;
mod input;
mod shortcut;

pub(super) use self::effect::Effect;

/// 把先行上屏的文本接到本次结果前面。
///
/// 为什么不像 macOS 那样「先上屏、再把这个键交给应用」：那要求上屏与放行严格有序。
/// IBus 的 `CommitText` 是走 D-Bus 发出去的，而 `ProcessKeyEvent` 回 false 让键走应用自己的路径，
/// 两条路没有顺序保证，应用可能先收到键再收到词。Windows 的 TSF 上这个顺序问题是实打实踩过的
/// （放行同步、上屏异步），所以两边都改成：本该放行的可打印键由我们连同前缀一起插入、把键吃掉。
fn with_prefix(prefix: Option<String>, effect: Effect, c: char) -> Effect {
    let Some(mut prefix) = prefix else {
        return effect;
    };
    match effect {
        Effect::Changed(commit) => {
            prefix.push_str(commit.as_deref().unwrap_or_default());
        }
        Effect::Navigated => {}
        Effect::Passthrough => prefix.push(c),
    }
    Effect::Changed(Some(prefix))
}
