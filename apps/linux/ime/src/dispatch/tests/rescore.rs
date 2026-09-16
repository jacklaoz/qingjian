//! 本地整句模型的节拍：没有模型时一切照旧，别让空转的节拍往 D-Bus 上发帧。
//!
//! 真模型（几十兆的 `.qjm`）不在仓库里，CI 上也没有，所以这里只测「没有模型」这一侧；
//! 带模型的重排本身在 `qingjian-core` 的 `engine::rescoring` 里测过。

use super::*;
use crate::dispatch::IDLE_TICK;

/// 没有模型：节拍就是闲着的一秒，敲键也不会把它变快。
#[test]
fn without_a_model_the_tick_stays_idle() {
    let mut router = router();
    assert_eq!(router.next_tick(), IDLE_TICK);
    router.type_str("nihao");
    assert_eq!(router.next_tick(), IDLE_TICK, "没有模型就没有防抖要等");
}

/// 空转的节拍不回帧：回了主循环就会白发一轮 D-Bus 信号，面板跟着闪。
#[test]
fn an_idle_tick_produces_no_frame() {
    let mut router = router();
    assert!(router.tick().is_none(), "没在组句");
    router.type_str("nihao");
    assert!(router.tick().is_none(), "在组句，但没有模型也没有云端词");
}

/// 模型文件不存在时 `configure_local_model` 不该挂上打分器，也不该 panic。
#[test]
fn a_missing_model_file_is_harmless() {
    let mut router = router();
    router.configure_local_model(None, &qingjian_platform::LocalModelConfig { enabled: true });
    assert!(!router.engine().has_sentence_scorer());
    router.type_str("nihao");
    assert_eq!(router.next_tick(), IDLE_TICK);
}

/// `[model] enabled = false` 只是不加载，别的照常。
#[test]
fn disabling_the_model_keeps_input_working() {
    let mut router = router();
    router.configure_local_model(
        None,
        &qingjian_platform::LocalModelConfig { enabled: false },
    );
    router.type_str("nihao");
    assert!(!router.page_texts().is_empty(), "候选照出");
}

/// 带真模型跑一遍：敲一段拼音，停一下，节拍应当把重排后的帧交出来。
///
/// 真词库与 54 MB 的模型都不在仓库里（CI 上也没有），所以这条默认不跑：
/// `cargo test -p qingjian-linux -- --ignored real_model` 在有产品数据的机器上手动跑。
#[test]
#[ignore = "要 data/generated/dict.qj 与 data/model/model.qjm"]
fn real_model_rescores_within_a_second() {
    let root = repo_root();
    let dict = root.join("data/generated/dict.qj");
    let model = root.join("data/model/model.qjm");
    assert!(dict.is_file() && model.is_file(), "产品数据不在，跳过");

    let spec = AssemblySpec {
        language_model: crate::assembly::LanguageModelFiles::find(&root.join("data/generated")),
        ..AssemblySpec::new(dict)
    };
    let engine = assembly::assemble(&spec).expect("真词库装得起来");
    let mut router = Router::new(engine, RouterConfig::default());
    router.configure_local_model(
        Some(model),
        &qingjian_platform::LocalModelConfig { enabled: true },
    );

    router.type_str("zhonghuarenmingongheguo");
    // 模型在后台加载要几百毫秒到几秒，**加载完之后的第一次查询**才会记下要打分的整句路径。
    // 真实使用里那就是用户接着敲的下一键（`recompose` 每次都先 `attach_loaded_model`），这里也照敲一键。
    let started = std::time::Instant::now();
    let mut typed_again = false;
    let mut rescored = None;
    while started.elapsed() < std::time::Duration::from_secs(20) {
        std::thread::sleep(router.next_tick());
        if let Some(frame) = router.tick() {
            rescored = Some(frame);
            break;
        }
        if router.engine().has_sentence_scorer() && !typed_again {
            typed_again = true;
            router.press('w');
        }
    }
    let frame = rescored.expect("二十秒内该重排完");
    assert!(!frame.candidates.items.is_empty(), "重排后仍有候选");
    assert!(router.engine().has_sentence_scorer(), "模型确实接上了");
}
