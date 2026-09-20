//! 几个输入上下文共用的那一条连接，外加边界上的毒化恢复与重连。
//!
//! 这是 IBus 走进程内引擎那一版里 `Shared`（`Arc<Mutex<Router>>`）的位置，
//! 换成了 `Arc<Mutex<Option<Connection>>>`：引擎搬到 Server 进程之后，
//! 前端这边要守的不再是引擎状态，而是**一条会断的连接**。
//!
//! 两件事在这里收口：
//!
//! - **锁不会被毒化拖死。** panic 时 `Mutex` 会被标记成 poisoned，之后每次 `lock()` 都返回错误；
//!   照 `expect` 写下去等于「崩过一次就再也打不了字」。一律取回里面的数据继续用。
//! - **连接断了自己接回来。** Server 重启、超时、协议出错都把连接扔掉，下一次按键重连；
//!   会话要重开，所以重连之后调用方拿到的会话编号会变（见 [`Shared::with`] 的说明）。

use std::sync::{Arc, Mutex, MutexGuard};

use super::Connection;
use crate::error::IbusError;

/// 共用的连接句柄。
#[derive(Clone, Default)]
pub struct Shared {
    /// 进程内唯一的连接；还没连上或刚断开时是 `None`。
    connection: Arc<Mutex<Option<Connection>>>,
}

impl Shared {
    /// 还没连上就先连，然后在连接上做一次事情。
    ///
    /// **出错就扔掉连接**：一问一答的协议上，一次失败之后收发就可能串位，与其猜不如重来。
    /// 调用方自己重开会话——这也是会话编号只在 [`Connection`] 内部递增、外面不缓存的原因。
    pub fn with<T>(
        &self,
        action: impl FnOnce(&mut Connection) -> Result<T, IbusError>,
    ) -> Result<T, IbusError> {
        let mut slot = self.lock();
        if slot.is_none() {
            *slot = Some(Connection::connect()?);
        }
        let connection = slot.as_mut().expect("刚连上");
        match action(connection) {
            Ok(value) => Ok(value),
            Err(error) => {
                tracing::warn!(%error, "与 Server 的连接出错，丢弃后下次重连");
                *slot = None;
                Err(error)
            }
        }
    }

    /// 现在连着吗（`--check` 与日志用）。
    pub fn connected(&self) -> bool {
        self.lock().is_some()
    }

    /// 拿锁。被毒化过就把里面的数据取回来接着用，不把整个输入法拖死。
    fn lock(&self) -> MutexGuard<'_, Option<Connection>> {
        self.connection.lock().unwrap_or_else(|poisoned| {
            tracing::error!("连接的锁被上一次 panic 毒化过，取回接着用");
            poisoned.into_inner()
        })
    }
}
