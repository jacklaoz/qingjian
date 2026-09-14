//! 青简 Linux 输入法壳（只做 Wayland）的入口。逻辑在库部分，这里只解析参数与启动。
//!
//! 现在是骨架：路径定位、按键翻译、自检就位，还没接上输入法框架——接法见
//! `docs/plan/linux_plan.md`，先走 IBus + 系统面板（GNOME / KDE 的终态），
//! 再给 wlroots 系补原生 `input-method-v2` 的自绘候选窗。

use qingjian_linux::check;
use tracing_subscriber::EnvFilter;

fn main() {
    init_logging();
    match std::env::args().nth(1).as_deref() {
        Some("--check") => {
            if !check::run() {
                std::process::exit(1);
            }
        }
        Some("--version") => println!("qingjian-linux {}", env!("CARGO_PKG_VERSION")),
        Some(other) => {
            eprintln!("不认识的参数：{other}");
            usage();
            std::process::exit(2);
        }
        // 还没有输入法服务可跑，先把自检当缺省行为，免得直接跑起来什么都不说
        None => {
            usage();
            println!();
            check::run();
        }
    }
}

fn usage() {
    println!("用法：qingjian-linux [--check | --version]");
    println!("  --check    找一遍用户目录、随包数据与配置，打印缺什么");
    println!("  --version  打印版本号");
}

/// 日志按 `RUST_LOG` 过滤，缺省 info；还没有输入法服务，先只写标准错误。
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
