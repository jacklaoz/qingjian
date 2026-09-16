//! 当前有键盘焦点的引擎对象是哪一个。
//!
//! 按键路径上 zbus 会把信号发出者（`SignalEmitter`）交到方法里，主循环没有这个东西——
//! 而本地整句模型重排完、云联想结果到了都要在按键之外重画候选窗。
//! 所以引擎对象在拿到焦点时把自己的对象路径记在这里，主循环照着它自己造一个发出者。
//!
//! 一个进程可以挂着多个引擎对象（每个输入上下文一个），但同一时刻只有一个有焦点，所以这里只记一个。

use std::sync::{Arc, Mutex};

use zbus::zvariant::OwnedObjectPath;

/// 共用的「当前聚焦的引擎对象路径」。
#[derive(Clone, Default)]
pub struct ActiveEngine {
    /// 聚焦对象的路径；没有焦点时为 `None`。
    path: Arc<Mutex<Option<OwnedObjectPath>>>,
}

impl ActiveEngine {
    /// 拿到焦点。
    pub fn set(&self, path: OwnedObjectPath) {
        *self.lock() = Some(path);
    }

    /// 失去焦点。只在记着的就是自己时才清：焦点可能已经给了下一个上下文，
    /// 那时旧对象的 `FocusOut` 才姗姗来迟，不能把新的抹掉。
    pub fn clear_if(&self, path: &OwnedObjectPath) {
        let mut current = self.lock();
        if current.as_ref() == Some(path) {
            *current = None;
        }
    }

    /// 当前聚焦的对象路径。
    pub fn path(&self) -> Option<OwnedObjectPath> {
        self.lock().clone()
    }

    /// 拿锁。与 [`Shared`](super::shared::Shared) 同一条规矩：毒化了就把里面的数据取回来接着用。
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<OwnedObjectPath>> {
        self.path.lock().unwrap_or_else(|poisoned| {
            tracing::error!("聚焦对象路径的锁被上一次 panic 毒化过，取回状态继续");
            poisoned.into_inner()
        })
    }
}
