//! `org.freedesktop.IBus.Factory`：ibus-daemon 每开一个输入上下文就叫它造一个引擎对象。

use std::sync::atomic::{AtomicU64, Ordering};

use zbus::object_server::ObjectServer;
use zbus::zvariant::OwnedObjectPath;

use super::engine::IBusEngine;
use super::shared::Shared;

/// 引擎对象挂在这个路径下，后面接一个自增序号。
const ENGINE_PATH_PREFIX: &str = "/org/freedesktop/IBus/Engine/Qingjian";

/// 工厂对象。
pub struct IBusFactory {
    /// 进程内唯一的 Router，造出来的引擎对象共用它。
    ///
    /// 一个进程服务多个输入上下文，但同一时刻只有一个有键盘焦点，所以共用一个 Router、
    /// 一份组句状态就够——这和 macOS 壳的进程级单例是同一个形状，
    /// 不同于 Windows Server 要按 `SessionId` 分派多会话（那边一个 Server 服务多个应用进程）。
    router: Shared,

    /// 引擎对象路径的序号。
    next: AtomicU64,
}

impl IBusFactory {
    pub fn new(router: Shared) -> Self {
        Self {
            router,
            next: AtomicU64::new(1),
        }
    }
}

#[zbus::interface(name = "org.freedesktop.IBus.Factory")]
impl IBusFactory {
    /// 造一个引擎对象，把它的路径回给 ibus-daemon。
    async fn create_engine(
        &self,
        name: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let serial = self.next.fetch_add(1, Ordering::Relaxed);
        let path = format!("{ENGINE_PATH_PREFIX}/{serial}");
        tracing::info!(engine = %name, %path, "造一个引擎对象");
        let object = OwnedObjectPath::try_from(path.clone())
            .map_err(|error| zbus::fdo::Error::Failed(format!("引擎对象路径不合法：{error}")))?;
        server
            .at(&object, IBusEngine::new(self.router.clone()))
            .await
            .map_err(|error| zbus::fdo::Error::Failed(format!("挂引擎对象失败：{error}")))?;
        Ok(object)
    }
}
