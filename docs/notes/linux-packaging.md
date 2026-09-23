# Linux 发行包：deb / rpm / flatpak（2026-09-23）

`apps/linux/packaging/build.sh` 一条命令打三种包，给别的 Linux 机器装着试。
这一页记三种包各装了什么、为什么这么分、怎么装，以及几处绕不过去的限制。
用户级的源码安装（`apps/linux/scripts/install.sh`）另见 [linux-fcitx5.md](linux-fcitx5.md) 与 [linux-dual-frontend.md](linux-dual-frontend.md)。

## 打包

```bash
apps/linux/packaging/build.sh               # 三种都打
apps/linux/packaging/build.sh deb rpm       # 只打给出的几种
QINGJIAN_SAMPLE=1 apps/linux/packaging/build.sh   # 没有产品数据时只带样例词库
```

产物在 `target/linux-pkg/out/`。只要 docker（免 sudo）和 flatpak，本机不用装 fcitx5 开发包或 rpmbuild：

| 步骤 | 在哪做 | 为什么 |
|---|---|---|
| Rust 的 Server 与 IBus 前端 | Ubuntu 22.04 容器，编一次三种包共用 | 这两个只依赖 libc / libm / libgcc；在 glibc 2.35 上编，实际门槛是 `GLIBC_2.34`。工具链与 crate 缓存直接挂本机的 `~/.cargo` / `~/.rustup`；依赖树里有 `openssl-sys`，编译要 `libssl-dev`（最终二进制并不链 libssl） |
| fcitx5 插件（deb） | Debian 13 容器 | 插件要对着目标发行版自己的 fcitx5 编；`CMakeLists.txt` 要 5.1.8+，Ubuntu 24.04 只有 5.1.7 |
| fcitx5 插件（rpm） | Fedora 42 容器 | 同上 |
| 随包资源 | 本机，`files.py resources` | 与 `install.sh` 同一套产品数据校验（`tools/release/data.lock`）与布局 |
| flatpak | 本机 flatpak-builder，runtime `org.freedesktop.Platform//25.08` | 装的是上面编好的二进制，不是 Flathub 那种离线从源码构建 |

## 三种包各装了什么

deb 与 rpm 都拆成两个包：

- **`qingjian`**：`/usr/bin/qingjian-linux-server`、`/usr/bin/qingjian-linux-ibus`、`/usr/share/qingjian/resources/`（词库等，
  Server 按「可执行文件上一层的 `share/qingjian/resources`」找到它）、`/usr/share/ibus/component/qingjian.xml`、
  `/usr/lib/systemd/user/qingjian-server.service`。装的时候 `systemctl --global enable` 让每个用户登录后自动起 Server。
- **`qingjian-fcitx5`**：fcitx5 插件与两份描述文件，放 fcitx5 自己的插件目录（Debian 是 `/usr/lib/<多架构>/fcitx5/`，Fedora 是 `/usr/lib64/fcitx5/`）。

IBus 前端没有额外依赖，跟 Server 同包；它的组件 XML 在系统目录，IBus 直接认，不用 `install.sh` 那套 environment.d。

**flatpak** 只有 IBus：`app.qingjian.Qingjian` 里是 Server、IBus 前端与资源。fcitx5 插件要装进宿主机的 fcitx5 进程，沙箱里的够不着。

## 装法

deb / rpm 装完 Server 就在已登录用户的会话里起来了（装包脚本对正在跑的用户 systemd 实例直接 restart）；
IBus 要重新登录一次（或 `ibus restart`）才看得到新引擎。flatpak 版的登记脚本也是当场起 Server。

### Debian / Ubuntu（deb）

```bash
sudo apt install ./qingjian_<版本>_amd64.deb                 # IBus 用户
sudo apt install ./qingjian_<版本>_amd64.deb ./qingjian-fcitx5_<版本>_amd64.deb   # fcitx5 用户
```

`qingjian` 只要 `libc6 (>= 2.34)`，Debian 12、Ubuntu 22.04 / 24.04 也装得上（IBus）；
`qingjian-fcitx5` 要 `libfcitx5core7 (>= 5.1.12)`，只有 Debian 13、Ubuntu 25.04 及以后。

### Fedora 42 及以后（rpm）

```bash
sudo dnf install ./qingjian-<版本>.x86_64.rpm
sudo dnf install ./qingjian-<版本>.x86_64.rpm ./qingjian-fcitx5-<版本>.x86_64.rpm
```

### 任何装了 flatpak、用 IBus 的发行版（flatpak）

```bash
flatpak install --user ./qingjian_<版本>_x86_64.flatpak
flatpak run app.qingjian.Qingjian | bash      # 在宿主机上登记（不用 sudo）
```

登记做三件事，都在用户目录：IBus 组件 XML、把它加进 `IBUS_COMPONENT_PATH` 的 environment.d 片段、
`ExecStart=flatpak run …` 的 systemd 用户服务（并立即启动）。卸载前先 `flatpak run app.qingjian.Qingjian --uninstall | bash`。

### 装完之后

- IBus：「设置 → 键盘 → 输入源」添加「青简」（GNOME），或 `ibus engine qingjian`。
- fcitx5：`fcitx5-configtool` 里取消「仅显示当前语言」，添加「青简」。
- 繁体：`~/.config/qingjian/config.toml` 的 `[general] traditional` 写 `taiwan` / `hongkong` / `standard`，
  再 `systemctl --user restart qingjian-server`（Linux Server 不热加载配置）；flatpak 版的配置在 `~/.var/app/app.qingjian.Qingjian/config/qingjian/`。
- 自检（不经过输入法框架，直接问 Server）：`qingjian-linux-ibus --check nihao`；
  flatpak 版是 `flatpak run --command=qingjian-ibus app.qingjian.Qingjian --check nihao`。
- Server 状态与日志：`systemctl --user status qingjian-server`、`journalctl --user -u qingjian-server`。

## 实测（2026-09-23，容器里装包 + 起 Server + `--check nihao`）

| 包 | 装在 | 结果 |
|---|---|---|
| deb 两个包 | Debian 13、Ubuntu 26.04 | 装上；插件不缺库；`systemctl --global` 自启登记上、卸载后清掉；自检出「你好」等候选 |
| deb `qingjian` | Ubuntu 24.04、22.04 | 同上（不装插件） |
| rpm 两个包 | Fedora 42、Fedora 最新、openSUSE Tumbleweed | 同上 |
| flatpak | 本机（Ubuntu 26.04） | 两个沙箱实例经共享目录的 socket 连通，自检出候选；沙箱里的 IBus 前端连上 `~/.cache/ibus` 下的私有 ibus-daemon 并注册了工厂；登记脚本语法通过 |

容器里没有图形会话，**真会话里切到青简打字没验**：这一步要在别的机器上装了之后手测，判据见 [linux-dual-frontend.md](linux-dual-frontend.md) 第二节。
flatpak 的宿主机登记脚本（写组件、environment.d、systemd 服务）也只验了语法，没在真机上跑过。

### fcitx5 的依赖与切换框架

`apt install ./…` / `dnf install ./…` 会把 fcitx5 与它的 GTK / Qt 输入法模块、配置工具一起装上（2026-09-23 在干净的
Debian 13、Ubuntu 26.04、Fedora 42 容器里看过；`dpkg -i` 不补依赖，要再跑一次 `sudo apt -f install`）。
**不会替你把输入法框架从 IBus 切到 fcitx5**，GNOME 默认是 IBus：

- Debian / Ubuntu：`im-config -n fcitx5`，重新登录；
- Fedora：装 `fcitx5-autostart`（`qingjian-fcitx5` 已经 Recommends 它，dnf 缺省会一起装），重新登录。

## 几处限制与坑

- **`systemctl --global enable` 不够**：它只对之后起来的用户 systemd 实例生效，而注销再登录得快时实例根本不重起。
  2026-09-23 在 Ubuntu 24.04 上踩过：10:51 装包、10:52 重新登录，用户实例是 10:46 起的一直没换，Server 一次都没起，
  切到青简打字没有候选。所以 postinst / `%post` 对 `loginctl list-users` 里的每个用户 `systemctl --user -M 用户@ restart`，
  prerm / `%preun` 同样 stop。
- **与用户级安装冲突**：`install.sh` 装在 `~/.local` 的那一份也叫 `qingjian-server.service`，组件名也相同。
  用户目录的单元优先于系统目录的，两份同时在时生效的是用户目录那份；装发行包之前先跑 `apps/linux/scripts/uninstall.sh`。
  flatpak 的登记脚本碰到这种情况会直接报错退出。
- **flatpak 的两个进程要共用一个 socket**：Server 与 IBus 前端是两个沙箱实例，各自的 `$XDG_RUNTIME_DIR` 不通。
  包里的 `qingjian-server` / `qingjian-ibus` 两个包装脚本把 `QINGJIAN_SOCKET` 指到 `$XDG_RUNTIME_DIR/app/$FLATPAK_ID/`，
  那是同一应用所有实例共享的目录。
- **flatpak 版的数据不在老地方**：配置、学习数据在 `~/.var/app/app.qingjian.Qingjian/`，与 deb / rpm / `install.sh` 的 `~/.config/qingjian`、`~/.local/share/qingjian` 不共用。
- **IBus 总线在沙箱里**：ibus-daemon 拉起引擎时设了 `IBUS_ADDRESS`，socket 在 `~/.cache/ibus`（也见过 `$XDG_RUNTIME_DIR/ibus`），
  清单给了这几处的只读权限；连 unix socket 不需要文件系统可写。
