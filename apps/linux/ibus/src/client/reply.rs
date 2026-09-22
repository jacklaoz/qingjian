//! Server 对一次事件的答复。

use qingjian_platform::protocol::{Frame, KeyOutcome, ServerMessage};
use serde_json::Value;

use crate::error::IbusError;

/// 一次事件之后 Server 让前端做的事。
///
/// 三样都由 Server 定：**前端不判断这个键该不该吃、该上什么屏**
/// （`docs/contributing.md`「架构约束」）。
#[derive(Debug)]
pub struct Reply {
    /// 这次按键吃掉还是放行给应用。
    pub outcome: KeyOutcome,

    /// 立刻上屏的文本；没有就是 `None`。
    pub commit: Option<String>,

    /// 要画的组句状态；空帧表示收起候选窗口。
    pub frame: Frame,
}

impl Default for Reply {
    /// 缺省是**放行**：认不出来的回包不能把用户的键吞掉，宁可让应用自己收到。
    fn default() -> Self {
        Self {
            outcome: KeyOutcome::Passthrough,
            commit: None,
            frame: Frame::default(),
        }
    }
}

impl Reply {
    /// 从 Server 的一条回包里读出来。
    ///
    /// 正常是 `KeyResult`；会话已经不在了 Server 回 `Ignored`（旧回调的延迟通知，当作没事发生）。
    pub fn from_value(value: Value) -> Result<Self, IbusError> {
        if value.get("Ignored").is_some() {
            return Ok(Self::default());
        }
        match serde_json::from_value::<ServerMessage>(value.clone()) {
            Ok(ServerMessage::KeyResult {
                outcome,
                commit,
                frame,
                ..
            }) => Ok(Self {
                outcome,
                commit,
                frame,
            }),
            Ok(ServerMessage::Update { frame, .. }) => Ok(Self {
                frame,
                ..Self::default()
            }),
            _ => Err(IbusError::Unexpected(value.to_string())),
        }
    }
}
