//! 云联想接入：关着、开着但没密钥、热加载改配置，三种情形都不能影响打字。
//!
//! 这里**不连网**：没有密钥 `CloudPredictor::new` 就失败，壳退回本地候选，测的正是这条退路。

use qingjian_predict::PredictConfig;

use super::*;

/// 一份「开着但一定拿不到密钥」的配置：环境变量名取一个不会有人设的。
fn enabled_without_key() -> PredictConfig {
    PredictConfig {
        enabled: true,
        api_key: None,
        api_key_env: "QINGJIAN_TEST_NO_SUCH_KEY".to_owned(),
        ..PredictConfig::default()
    }
}

/// 缺省就是关着的，接上去也还是关着。
#[test]
fn disabled_by_default() {
    let mut router = router();
    router.configure_cloud(&PredictConfig::default());
    assert!(!router.engine().prediction_enabled());
}

/// 开着但没密钥：退回本地候选，不能 panic，也不能把候选弄没。
#[test]
fn enabled_without_a_key_falls_back() {
    let mut router = router();
    router.configure_cloud(&enabled_without_key());
    assert!(!router.engine().prediction_enabled(), "没密钥就不该算启用");
    router.type_str("nihao");
    assert!(!router.page_texts().is_empty(), "候选照出");
}

/// 热加载：`[predict]` 没变就不动它，变了才重建。
#[test]
fn reload_only_rebuilds_when_the_section_changes() {
    let mut router = router();
    router.configure_cloud(&PredictConfig::default());
    router.type_str("nihao");
    // 同一份配置再来一次：什么都不该发生
    router.apply_predict_config(&PredictConfig::default());
    assert!(!router.engine().prediction_enabled());
    // 换成开着但没密钥的：仍然退回本地候选，组句不受影响
    router.apply_predict_config(&enabled_without_key());
    assert!(!router.engine().prediction_enabled());
    assert!(!router.page_texts().is_empty(), "热加载不该把候选弄没");
}

/// 云端候选没接上时，节拍不该因为它回帧——回了主循环就会白发一轮 D-Bus 信号。
#[test]
fn ticks_stay_quiet_without_cloud() {
    let mut router = router();
    router.configure_cloud(&enabled_without_key());
    router.type_str("nihao");
    assert!(router.tick().is_none());
}

/// 接上一个**本机假接口**跑一遍：云端词要能并进候选布局，并且由节拍交出新的一帧。
///
/// 不连网也不需要密钥：假接口就在 127.0.0.1 上，按 OpenAI 兼容格式回一条固定答复。
/// 验的是壳这一侧的接线——请求发得出去、结果收得回来、收回来之后这一帧变了
/// （主循环正是按 `tick()` 回不回帧来决定要不要重画候选窗的）。
#[test]
fn cloud_words_reach_the_frame() {
    // 模型答复的正文本身是一段 JSON；本地第一条候选会被丢掉，所以给一个词库里排不到第一的词
    let reply = r#"{"words":[{"text":"拟好","pinyin":"ni hao"}],"sentence":"你好呀"}"#;
    let base_url = spawn_fake_api(reply);

    let mut router = router();
    router.configure_cloud(&PredictConfig {
        enabled: true,
        base_url,
        api_key: Some("test-key".to_owned()),
        // 防抖是给真人敲键用的，测试里没必要等
        debounce_ms: 10,
        ..PredictConfig::default()
    });
    assert!(router.engine().prediction_enabled(), "有密钥就该启用");

    router.type_str("nihao");
    let started = std::time::Instant::now();
    let mut cloud_frame = None;
    while started.elapsed() < std::time::Duration::from_secs(10) {
        std::thread::sleep(std::time::Duration::from_millis(20));
        if let Some(frame) = router.tick() {
            cloud_frame = Some(frame);
            break;
        }
    }
    let frame = cloud_frame.expect("十秒内该收到云端结果并交出新的一帧");
    let texts: Vec<&str> = frame
        .candidates
        .items
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect();
    assert!(
        texts.contains(&"拟好"),
        "云端词该并进候选；这一页是 {texts:?}"
    );
    assert_eq!(
        frame.sentence.as_deref(),
        Some("你好呀"),
        "整句补全该记下来"
    );
}

/// 起一个只会回同一条答复的 OpenAI 兼容接口，返回它的 base_url。
///
/// 手写 HTTP 是因为这点活不值得引一个 HTTP 服务端依赖：读完请求、回一段定长 JSON、关连接。
fn spawn_fake_api(reply_content: &str) -> String {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("绑本机端口");
    let address = listener.local_addr().expect("本机地址");
    // 答复正文是 JSON 里的一个字符串字段，引号要转义
    let escaped = reply_content.replace('"', "\\\"");
    let body = format!(
        r#"{{"id":"chatcmpl-test","object":"chat.completion","created":1,"model":"test","choices":[{{"index":0,"message":{{"role":"assistant","content":"{escaped}"}},"finish_reason":"stop"}}]}}"#
    );
    std::thread::Builder::new()
        .name("qingjian-fake-api".to_owned())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                // 请求体不读完就回应答，客户端那边会看到连接被重置
                let mut received = Vec::new();
                let mut buffer = [0_u8; 4096];
                while let Ok(read) = stream.read(&mut buffer) {
                    if read == 0 {
                        break;
                    }
                    received.extend_from_slice(&buffer[..read]);
                    if is_complete(&received) {
                        break;
                    }
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        })
        .expect("起假接口线程");
    format!("http://{address}")
}

/// 请求头收全了、而且正文按 `Content-Length` 也收全了没有。
fn is_complete(received: &[u8]) -> bool {
    let Some(headers_end) = received
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
    else {
        return false;
    };
    let headers = String::from_utf8_lossy(&received[..headers_end]).to_lowercase();
    let length: usize = headers
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0);
    received.len() >= headers_end + length
}
