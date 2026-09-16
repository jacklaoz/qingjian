//! `org.freedesktop.IBus.Engine`：ibus-daemon 每开一个输入上下文就叫工厂造一个这个对象。
//!
//! 这一层只做两件事：把 D-Bus 来的按键翻成 [`KeyInput`] 交给 Router，把 Router 回的帧
//! 经 [`View`] 折成 IBus 面板认的三样（preedit / 候选表 / 辅助行）再发信号出去。
//! **不做任何排序、查词或文本变换**——那些在 Core 里。

use qingjian_platform::protocol::KeyOutcome;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

use super::shared::Shared;
use super::variant;
use super::view::View;
use crate::keys::{KeyInput, modifiers};

/// `UpdatePreeditText` 的 `mode`：焦点移走时把 preedit 清掉（ibus 的 `IBUS_ENGINE_PREEDIT_CLEAR`）。
/// 青简在 `FocusOut` 里自己把缓冲原样上屏，不靠 ibus 替它提交，所以是 CLEAR 不是 COMMIT。
const PREEDIT_CLEAR: u32 = 0;

/// 一个输入上下文的引擎对象。
///
/// Router 放在 `Mutex` 里而不是另开一条工人线程：`Router` 是 `Send`（只是因为 Engine 内部那几个
/// `RefCell` 缓存不是 `Sync`），加把锁串行访问就满足 zbus 对接口对象 `Send + Sync` 的要求。
/// 锁的毒化恢复与 panic 隔离在 [`Shared`]。
pub struct IBusEngine {
    /// 进程内唯一的 Router，几个引擎对象共用（同一时刻只有一个上下文有焦点）。
    router: Shared,
}

impl IBusEngine {
    pub fn new(router: Shared) -> Self {
        Self { router }
    }

    /// 把一帧发给面板：preedit、候选表、辅助行各发一次，空的就发隐藏。
    async fn present(&self, emitter: &SignalEmitter<'_>) -> zbus::Result<()> {
        let frame = self.router.lock().current_frame();
        let view = View::from_frame(&frame);

        if view.preedit.is_empty() {
            Self::hide_preedit_text(emitter).await?;
            Self::update_preedit_text(emitter, variant::text(""), 0, false, PREEDIT_CLEAR).await?;
        } else {
            let text = variant::segmented_text(&view.preedit);
            Self::update_preedit_text(emitter, text, view.cursor, true, PREEDIT_CLEAR).await?;
            Self::show_preedit_text(emitter).await?;
        }

        if view.has_candidates() {
            let table = variant::lookup_table(&view.candidates, view.cursor_index, true);
            Self::update_lookup_table(emitter, table, true).await?;
            Self::show_lookup_table(emitter).await?;
        } else {
            Self::update_lookup_table(emitter, variant::lookup_table(&[], 0, false), false).await?;
            Self::hide_lookup_table(emitter).await?;
        }

        match &view.auxiliary {
            Some(line) => {
                Self::update_auxiliary_text(emitter, variant::text(line), true).await?;
                Self::show_auxiliary_text(emitter).await?;
            }
            None => {
                Self::update_auxiliary_text(emitter, variant::text(""), false).await?;
                Self::hide_auxiliary_text(emitter).await?;
            }
        }
        Ok(())
    }

    /// 有要上屏的文本就发出去。
    async fn commit(&self, emitter: &SignalEmitter<'_>, text: Option<String>) -> zbus::Result<()> {
        let Some(text) = text.filter(|t| !t.is_empty()) else {
            return Ok(());
        };
        Self::commit_text(emitter, variant::text(&text)).await
    }
}

#[zbus::interface(name = "org.freedesktop.IBus.Engine")]
impl IBusEngine {
    /// 一次按键。回 `true` 表示吃掉，`false` 让应用自己处理。
    async fn process_key_event(
        &mut self,
        keyval: u32,
        keycode: u32,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        // 抬键一律放行：输入法只认按下
        if modifiers::is_release(state) {
            return false;
        }
        // 按键是最容易踩到边界情况的地方，拦一层 panic：拦下后组句已清干净，这个键让给应用
        let Some(response) = self.router.guarded("按键", |router| {
            router.handle_key(KeyInput::new(keyval, keycode, state))
        }) else {
            let _ = self.present(&emitter).await;
            return false;
        };
        let consumed = response.outcome == KeyOutcome::Consumed;
        if let Err(error) = self.commit(&emitter, response.commit).await {
            tracing::warn!(%error, "上屏失败");
        }
        if let Err(error) = self.present(&emitter).await {
            tracing::warn!(%error, "更新候选窗失败");
        }
        consumed
    }

    /// 拿到焦点。
    async fn focus_in(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        tracing::debug!("拿到焦点");
        let _ = self.present(&emitter).await;
    }

    /// 焦点离开：缓冲原样上屏，与 macOS 的 `commitComposition`、Windows 的 `Commit` 一致。
    async fn focus_out(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        let text = self.router.lock().commit_raw();
        tracing::debug!(?text, "焦点离开，结束组句");
        let _ = self.commit(&emitter, text).await;
        let _ = self.present(&emitter).await;
    }

    /// IBus 1.5 之后带对象路径与客户端名的那一对，行为同上。
    async fn focus_in2(
        &mut self,
        _object_path: String,
        client: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        tracing::debug!(%client, "拿到焦点");
        let _ = self.present(&emitter).await;
    }

    async fn focus_out2(
        &mut self,
        _object_path: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.focus_out(emitter).await;
    }

    /// 应用要求重置：丢掉组句不上屏。
    async fn reset(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.router.lock().reset_composition();
        let _ = self.present(&emitter).await;
    }

    /// 输入法被启用 / 停用。
    async fn enable(&mut self) {
        tracing::debug!("启用");
    }

    async fn disable(&mut self) {
        self.router.lock().reset_composition();
        tracing::debug!("停用");
    }

    /// 下面这些 IBus 会调但我们不处理：翻页由 Core 的候选布局算（面板自己翻会和它打架），
    /// 光标位置在方案 C 下用不着（面板自己定位），属性栏还没做。声明出来是因为
    /// 不实现的话 ibus-daemon 每次调用都会记一条错误。
    async fn set_capabilities(&mut self, _capabilities: u32) {}

    async fn set_cursor_location(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    async fn page_up(&mut self) {}

    async fn page_down(&mut self) {}

    async fn cursor_up(&mut self) {}

    async fn cursor_down(&mut self) {}

    async fn candidate_clicked(&mut self, _index: u32, _button: u32, _state: u32) {}

    async fn property_activate(&mut self, _name: String, _state: u32) {}

    async fn destroy(&mut self) {
        tracing::debug!("引擎对象销毁");
    }

    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    /// 参数是 `(vubu)`，**最后那个 `mode` 不能省**：ibus-daemon 按 `(vubu)` 解这条信号，
    /// 少一个参数它解不出来，只在自己的日志里留一条 `arg0 != NULL` 的 CRITICAL，
    /// 然后把整条信号丢掉——症状就是拼音行不显示，而且这边一点错都看不到。
    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_lookup_table(
        emitter: &SignalEmitter<'_>,
        table: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_lookup_table(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_lookup_table(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_auxiliary_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_auxiliary_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_auxiliary_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}
