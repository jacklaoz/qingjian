//! 一次按键交出去的结果。

use qingjian_platform::protocol::{Frame, KeyOutcome};

/// Router 处理完一次按键后要让输入法框架做的事。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyResponse {
    /// 这次按键吃掉还是放行（IBus 的 `ProcessKeyEvent` 据此回 true / false）。
    pub outcome: KeyOutcome,

    /// 本次要立即上屏的文本；没有则为 `None`。
    pub commit: Option<String>,

    /// 处理后要画的组句状态；空 [`Frame`] 表示收起候选。
    pub frame: Frame,
}
