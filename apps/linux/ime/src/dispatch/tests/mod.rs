//! Router 的闭环测试：不经 IBus，直接把按键喂给 Router 看它回什么。
//! 用仓库自带的样例词库，所以在没有产品数据的机器上（CI）也跑得了。

mod chinese;
mod english;
mod function;
mod shortcut;

use std::path::PathBuf;

use qingjian_platform::protocol::{KeyModifiers, KeyOutcome};

use super::{KeyResponse, Router, RouterConfig};
use crate::assembly::{self, AssemblySpec};
use crate::keys::{KeyInput, keysym};

/// Caps Lock 亮着。
const CAPS: u32 = 1 << 1;

/// Shift 按着。
const SHIFT: u32 = 1 << 0;

/// Control 按着。
const CONTROL: u32 = 1 << 2;

/// Mod1（Alt）按着。
const ALT: u32 = 1 << 3;

/// 主键盘区 `1` 键的硬件键码。
const KEYCODE_1: u32 = 10;

/// 仓库根：测试的工作目录是 crate 目录，样例数据在仓库根下。
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("仓库根")
}

/// 用样例词库装一个 Router。不给用户目录，所以学习只在内存里，不碰真实文件。
fn router() -> Router {
    router_with(RouterConfig::default())
}

fn router_with(config: RouterConfig) -> Router {
    let root = repo_root();
    let spec = AssemblySpec {
        english: Some(root.join("assets/sample/english.tsv")),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    };
    let engine = assembly::assemble(&spec).expect("样例词库装得起来");
    Router::new(engine, config)
}

impl Router {
    /// 敲一个可见字符（keysym 就是码点）。
    fn press(&mut self, c: char) -> KeyResponse {
        self.press_with(c, 0)
    }

    fn press_with(&mut self, c: char, state: u32) -> KeyResponse {
        self.handle_key(KeyInput::from_keysym(c as u32, state))
    }

    /// 敲一个功能键。
    fn press_key(&mut self, keysym: u32) -> KeyResponse {
        self.handle_key(KeyInput::from_keysym(keysym, 0))
    }

    /// 敲一串字母。
    fn type_str(&mut self, text: &str) -> KeyResponse {
        let mut last = None;
        for c in text.chars() {
            last = Some(self.press(c));
        }
        last.expect("至少一个字符")
    }

    /// 当前帧里这一页的候选文本。
    fn page_texts(&self) -> Vec<String> {
        self.current_frame()
            .candidates
            .items
            .iter()
            .map(|c| c.text.clone())
            .collect()
    }

    /// 当前 preedit 拼起来的整行。
    fn preedit(&self) -> String {
        self.current_frame()
            .preedit
            .iter()
            .map(|s| s.text.as_str())
            .collect()
    }
}

/// 修饰键组合，拿来和配置里的快捷键比。
fn chord(state: u32) -> KeyModifiers {
    crate::keys::modifiers::from_mask(state).chord()
}

#[test]
fn sample_dictionary_assembles() {
    let router = router();
    assert!(router.engine().dictionary().len() > 100, "样例词库装上了");
}

/// 没在组句时帧是空的：IBus 侧据此收起候选。
#[test]
fn idle_frame_is_empty() {
    let router = router();
    assert!(router.current_frame().is_empty());
}

/// 带 Ctrl / Alt / Super 而没配到快捷键的键一律归应用。
#[test]
fn command_chords_pass_through() {
    let mut router = router();
    assert_eq!(
        router.press_with('c', CONTROL).outcome,
        KeyOutcome::Passthrough
    );
    assert_eq!(router.press_with('x', ALT).outcome, KeyOutcome::Passthrough);
    assert!(router.current_frame().is_empty(), "没有进组句");
}

/// 配置热加载：改了每页候选数，分页跟着变。
#[test]
fn page_size_reloads() {
    let mut router = router_with(RouterConfig {
        page_size: 3,
        ..RouterConfig::default()
    });
    router.type_str("ni");
    assert!(router.page_texts().len() <= 3, "一页最多三个");
    router.set_config(RouterConfig {
        page_size: 9,
        ..RouterConfig::default()
    });
    assert_eq!(router.config().page_size, 9);
}

/// `page_size` 给 0 会让分页除零，构造时夹到至少 1。
#[test]
fn zero_page_size_is_clamped() {
    let router = router_with(RouterConfig {
        page_size: 0,
        ..RouterConfig::default()
    });
    assert_eq!(router.config().page_size, 1);
}

/// zbus 的接口对象要求 `Send + Sync`。`Engine` 有五个 `RefCell` 缓存所以不是 `Sync`，
/// 但只要它是 `Send`，套一层 `Mutex` 就能满足，不必再开一条工人线程来持有它。
/// 这条在编译期就会红，是 D-Bus 那层怎么搭的前提。
#[test]
fn router_is_send_so_a_mutex_makes_it_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<Router>();
    assert_sync::<std::sync::Mutex<Router>>();
}
