//! 青简 Linux 输入法壳（只做 Wayland）的入口。逻辑在库部分，这里只解析参数与启动。
//!
//! 接入走 IBus（方案 C，GNOME / KDE 的 Wayland 会话上的终态）；
//! wlroots 系的原生 `input-method-v2` 自绘还没做，见 `docs/plan/linux_plan.md`。

use qingjian_linux::{check, ibus, run};
use tracing_subscriber::EnvFilter;

fn main() {
    init_logging();
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--ibus") => start(),
        Some("--check") => {
            if !check::run() {
                std::process::exit(1);
            }
        }
        Some("--ibus-xml") => {
            let exec = args.next().unwrap_or_else(default_exec);
            print!("{}", ibus::component::xml(&exec));
        }
        Some("--version") => println!("qingjian-linux {}", env!("CARGO_PKG_VERSION")),
        Some(other) => {
            eprintln!("不认识的参数：{other}");
            usage();
            std::process::exit(2);
        }
        None => {
            usage();
            println!();
            check::run();
        }
    }
}

/// 起 IBus 服务。tokio 运行时只在这条路上建，`--check` 那条不需要。
fn start() {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("起不了异步运行时：{error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = runtime.block_on(run::run()) {
        tracing::error!(%error, "青简退出");
        eprintln!("青简退出：{error}");
        std::process::exit(1);
    }
}

/// `--ibus-xml` 没给路径时用当前可执行文件的绝对路径。
fn default_exec() -> String {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "qingjian-linux".to_owned())
}

fn usage() {
    println!("用法：qingjian-linux [--ibus | --check | --ibus-xml [路径] | --version]");
    println!("  --ibus          作为 IBus 引擎运行（ibus-daemon 按组件 XML 拉起）");
    println!("  --check         找一遍用户目录、随包数据与配置，打印缺什么");
    println!("  --ibus-xml      打印 IBus 组件 XML（装包时放 /usr/share/ibus/component/）");
    println!("  --version       打印版本号");
}

/// 日志按 `RUST_LOG` 过滤，缺省 info；ibus-daemon 会把 stderr 收进它自己的日志。
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
