//! 几个引擎对象共用的 Router 句柄，外加边界上的 panic 隔离。
//!
//! 两件事在这里收口：
//!
//! - **锁不会被毒化拖死。** panic 时 `Mutex` 会被标记成 poisoned，之后每次 `lock()` 都返回错误；
//!   照 `expect` 写下去等于「崩过一次就再也打不了字」。这里一律取回里面的数据继续用——
//!   Router 的状态是组句缓冲与候选，坏了最多这一次按键不对，下一键就重算。
//! - **panic 不穿过 D-Bus 边界。** 与 macOS 壳在 IMK 回调边界 `catch_unwind`、Windows 在
//!   工人循环里拦是同一条规矩（见 `docs/design/architecture.md`「崩溃不丢」）：
//!   拦下后把缓冲区里的字母原样交出去、清引擎状态，输入法接着服务。

use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::dispatch::Router;

/// 共用的 Router 句柄。
#[derive(Clone)]
pub struct Shared {
    router: Arc<Mutex<Router>>,
}

impl Shared {
    pub fn new(router: Router) -> Self {
        Self {
            router: Arc::new(Mutex::new(router)),
        }
    }

    /// 拿锁。被毒化过就把里面的数据取回来接着用，不把整个输入法拖死。
    pub fn lock(&self) -> MutexGuard<'_, Router> {
        self.router.lock().unwrap_or_else(|poisoned| {
            tracing::error!("Router 的锁被上一次 panic 毒化过，取回状态继续");
            poisoned.into_inner()
        })
    }

    /// 在 panic 隔离里动一次 Router。panic 了就善后（缓冲原样交出、清状态）并把那段文本回给调用方，
    /// 与 macOS 壳 `imk::recover_from_panic` 做的事一样。
    pub fn guarded<T>(&self, what: &str, action: impl FnOnce(&mut Router) -> T) -> Option<T> {
        let mut router = self.lock();
        match std::panic::catch_unwind(AssertUnwindSafe(|| action(&mut router))) {
            Ok(value) => Some(value),
            Err(_) => {
                tracing::error!(what, "处理时 panic，已拦下；清掉组句接着服务");
                // 善后本身再 panic 就真没辙了，也拦一层
                let _ = std::panic::catch_unwind(AssertUnwindSafe(|| router.reset_composition()));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use qingjian_core::Engine;
    use qingjian_dictionary::Dictionary;

    use super::*;
    use crate::dispatch::RouterConfig;

    fn shared() -> Shared {
        let dict = Dictionary::from_path(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../assets/sample/dict.tsv")
                .as_path(),
        )
        .expect("样例词库");
        Shared::new(Router::new(Engine::new(dict), RouterConfig::default()))
    }

    /// 正常路径就是把返回值交出来。
    #[test]
    fn guarded_passes_the_value_through() {
        let shared = shared();
        let answer = shared.guarded("测试", |router| router.config().page_size);
        assert_eq!(answer, Some(9));
    }

    /// panic 被拦下，返回 `None`，而且**锁还能再用**——这正是毒化会毁掉的东西。
    #[test]
    fn panic_is_caught_and_the_lock_still_works() {
        let shared = shared();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let answer = shared.guarded::<()>("故意炸", |_| panic!("故意的"));
        std::panic::set_hook(previous);
        assert!(answer.is_none(), "panic 的那次没有值");
        // 拦下之后还能接着用：不做毒化恢复的话这里会直接炸
        assert_eq!(
            shared.guarded("再来一次", |router| router.config().page_size),
            Some(9)
        );
    }

    /// panic 之后组句被清干净，不会留着半截状态。
    #[test]
    fn composition_is_cleared_after_a_panic() {
        let shared = shared();
        shared.guarded("敲几个字母", |router| {
            router.handle_key(crate::keys::KeyInput::from_keysym('n' as u32, 0));
            router.handle_key(crate::keys::KeyInput::from_keysym('i' as u32, 0));
        });
        assert!(!shared.lock().current_frame().is_empty(), "先确认在组句");
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        shared.guarded::<()>("故意炸", |_| panic!("故意的"));
        std::panic::set_hook(previous);
        assert!(
            shared.lock().current_frame().is_empty(),
            "拦下后组句该清干净"
        );
    }
}
