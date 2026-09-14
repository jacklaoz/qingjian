//! 连上 IBus 总线并把工厂挂上去。
//!
//! IBus 不走会话总线，自己起一条。地址三处找：ibus-daemon 拉起我们时设的 `IBUS_ADDRESS`
//! 环境变量（正常情况走这条）、`ibus address` 会读的那个地址文件、以及用户手动指定的。

use std::path::{Path, PathBuf};

use zbus::connection::Builder;

use super::component::COMPONENT_NAME;
use super::factory::IBusFactory;
use super::shared::Shared;

/// 工厂对象挂在这个路径，IBus 写死的。
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

/// 连上总线、挂好工厂、占住组件名。返回的连接活着服务就活着。
pub async fn serve(router: Shared) -> Result<zbus::Connection, zbus::Error> {
    let address = bus_address().ok_or_else(|| {
        zbus::Error::Address("找不到 IBus 总线地址（IBUS_ADDRESS 没设，地址文件也没有）".to_owned())
    })?;
    tracing::info!(%address, "连 IBus 总线");
    let connection = Builder::address(address.as_str())?
        .serve_at(FACTORY_PATH, IBusFactory::new(router))?
        .name(COMPONENT_NAME)?
        .build()
        .await?;
    tracing::info!(name = COMPONENT_NAME, path = FACTORY_PATH, "工厂已就绪");
    Ok(connection)
}

/// IBus 总线地址。
pub fn bus_address() -> Option<String> {
    if let Some(address) = std::env::var_os("IBUS_ADDRESS")
        && !address.is_empty()
    {
        return address.into_string().ok();
    }
    address_from_file()
}

/// 从 `~/.config/ibus/bus/` 下的地址文件里读。
///
/// 文件名是 ibus **算出来**的：`<机器 id>-<主机>-<显示标识>`。不能扫目录随便挑一个——
/// 那里常年躺着别的输入法或上一次会话留下的陈旧文件（真机上见过 fcitx 留的 `-unix-0` 指着会话总线），
/// 按字典序挑会挑中死的那个，连上去一个信号都收不到。所以先按规则算文件名，
/// 算出来的不在才退回扫目录，且只认 `IBUS_DAEMON_PID` 还活着的。
fn address_from_file() -> Option<String> {
    let dir = bus_dir()?;
    if let Some(name) = socket_name()
        && let Some(address) = read_address(&dir.join(name))
    {
        return Some(address);
    }
    tracing::debug!("按规则算的地址文件不在，退回扫目录");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
        .iter()
        .find(|path| daemon_alive(path))
        .and_then(|path| read_address(path))
}

/// ibus 算地址文件名的规矩（见 ibus 的 `ibus_get_socket_path`）：
/// Wayland 会话用 `WAYLAND_DISPLAY` 当显示标识、主机名固定 `unix`；
/// X11 会话从 `DISPLAY` 里拆出主机与显示号。青简只做 Wayland，X11 那支是给
/// 「Wayland 会话里跑着 XWayland、`WAYLAND_DISPLAY` 没传进来」这种情况兜底的。
fn socket_name() -> Option<String> {
    let machine = machine_id()?;
    let (host, display) = display_id()?;
    Some(format!("{machine}-{host}-{display}"))
}

/// `(主机名, 显示标识)`。
fn display_id() -> Option<(String, String)> {
    if let Some(wayland) = std::env::var_os("WAYLAND_DISPLAY")
        && !wayland.is_empty()
    {
        return Some(("unix".to_owned(), wayland.into_string().ok()?));
    }
    let display = std::env::var("DISPLAY").ok()?;
    let (host, rest) = display.split_once(':')?;
    // `:0.1` 的屏幕号不进文件名
    let number = rest.split('.').next()?.to_owned();
    let host = if host.is_empty() {
        "unix".to_owned()
    } else {
        host.to_owned()
    };
    Some((host, number))
}

/// 这个地址文件记的 daemon 还活着吗。陈旧文件的 PID 早就没了。
fn daemon_alive(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let pid = content
        .lines()
        .find_map(|line| line.strip_prefix("IBUS_DAEMON_PID="))
        .and_then(|pid| pid.trim().parse::<u32>().ok());
    match pid {
        Some(pid) => Path::new(&format!("/proc/{pid}")).is_dir(),
        // 没记 PID 就不敢说它死了
        None => true,
    }
}

/// 地址文件是几行 `键=值`，要的是 `IBUS_ADDRESS`。
fn read_address(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        line.strip_prefix("IBUS_ADDRESS=")
            .map(|address| address.trim().to_owned())
    })
}

/// 地址文件所在目录 `$XDG_CONFIG_HOME/ibus/bus`。
fn bus_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(value) if PathBuf::from(&value).is_absolute() => PathBuf::from(value),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    let dir = base.join("ibus/bus");
    dir.is_dir().then_some(dir)
}

fn machine_id() -> Option<String> {
    for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(id) = std::fs::read_to_string(path) {
            let id = id.trim().to_owned();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 环境变量优先：ibus-daemon 拉起引擎进程时设的就是它。
    #[test]
    fn env_address_wins() {
        unsafe { std::env::set_var("IBUS_ADDRESS", "unix:abstract=/tmp/test-ibus") };
        assert_eq!(
            bus_address().as_deref(),
            Some("unix:abstract=/tmp/test-ibus")
        );
        unsafe { std::env::remove_var("IBUS_ADDRESS") };
    }

    /// 地址文件里只认 `IBUS_ADDRESS=` 那一行，别的键（IBUS_DAEMON_PID）不要。
    #[test]
    fn address_file_picks_the_right_key() {
        let dir = std::env::temp_dir().join("qingjian-ibus-address-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("machine-unix-0");
        std::fs::write(
            &path,
            "IBUS_DAEMON_PID=1234\nIBUS_ADDRESS=unix:abstract=/tmp/dbus-x\n",
        )
        .unwrap();
        assert_eq!(
            read_address(&path).as_deref(),
            Some("unix:abstract=/tmp/dbus-x")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Wayland 会话的文件名：主机名固定 `unix`，显示标识就是 `WAYLAND_DISPLAY`。
    #[test]
    fn wayland_display_id() {
        unsafe { std::env::set_var("WAYLAND_DISPLAY", "wayland-0") };
        assert_eq!(
            display_id(),
            Some(("unix".to_owned(), "wayland-0".to_owned()))
        );
        unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
    }

    /// X11 兜底：`:0` 的主机名为空要补成 `unix`，`.1` 的屏幕号不进文件名。
    #[test]
    fn x11_display_id_drops_the_screen_number() {
        unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
        unsafe { std::env::set_var("DISPLAY", ":0.1") };
        assert_eq!(display_id(), Some(("unix".to_owned(), "0".to_owned())));
        unsafe { std::env::set_var("DISPLAY", "box:2") };
        assert_eq!(display_id(), Some(("box".to_owned(), "2".to_owned())));
        unsafe { std::env::remove_var("DISPLAY") };
    }

    /// 记着的 PID 不在了就当这个地址文件是陈旧的——真机上见过 fcitx 留的文件躺在那里，
    /// 按字典序还排在当前会话的前面。
    #[test]
    fn stale_address_files_are_recognised() {
        let dir = std::env::temp_dir().join("qingjian-ibus-stale-test");
        std::fs::create_dir_all(&dir).unwrap();
        let stale = dir.join("machine-unix-0");
        // PID 取一个几乎不可能存在的值
        std::fs::write(
            &stale,
            "IBUS_ADDRESS=unix:path=/dead\nIBUS_DAEMON_PID=4194300\n",
        )
        .unwrap();
        assert!(!daemon_alive(&stale));
        let live = dir.join("machine-unix-wayland-0");
        std::fs::write(
            &live,
            format!(
                "IBUS_ADDRESS=unix:path=/live\nIBUS_DAEMON_PID={}\n",
                std::process::id()
            ),
        )
        .unwrap();
        assert!(daemon_alive(&live), "自己这个进程当然活着");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 文件里没有那一行时不能瞎猜。
    #[test]
    fn address_file_without_the_key_is_none() {
        let dir = std::env::temp_dir().join("qingjian-ibus-address-empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("machine-unix-0");
        std::fs::write(&path, "IBUS_DAEMON_PID=1234\n").unwrap();
        assert!(read_address(&path).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
