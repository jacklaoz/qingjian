//! 青简 Linux 设置界面。
//!
//! 与输入法是两个可执行文件：输入法进程不该扛着一整套 GUI 依赖。
//! 两边通过 `~/.config/qingjian/config.toml` 对话——这里写，输入法每秒看一次 mtime 热加载，
//! 不用 IPC，也就不用管对方在不在跑。

mod app;
mod form;
mod pages;
mod settings;

use gtk::Application;
use gtk::prelude::*;
use tracing_subscriber::EnvFilter;

/// D-Bus 上的应用标识，与桌面文件同名。
const APP_ID: &str = "app.qingjian.Settings";

fn main() -> gtk::glib::ExitCode {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let application = Application::builder().application_id(APP_ID).build();
    application.connect_activate(app::build);
    application.run()
}
