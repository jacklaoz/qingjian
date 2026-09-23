# Linux 发行包：deb / rpm（2026-09-23）

`apps/linux/packaging/build.sh` 一条命令打 deb 与 rpm，前端是 fcitx5 插件，给别的 Linux 机器装着试。
这一页记包里装了什么、为什么这么分、怎么装、怎么验，以及几处限制。
用户级的源码安装（`apps/linux/scripts/install.sh`）另见 [linux-fcitx5.md](linux-fcitx5.md)。

## 打包

```bash
apps/linux/packaging/build.sh               # deb 与 rpm 都打
apps/linux/packaging/build.sh deb           # 只打给出的那种
QINGJIAN_SAMPLE=1 apps/linux/packaging/build.sh   # 没有产品数据时只带样例词库
```

产物在 `target/linux-pkg/out/`。只要 docker（免 sudo），本机不用装 fcitx5 开发包或 rpmbuild：

| 步骤 | 在哪做 | 为什么 |
|---|---|---|
| Rust 的 Server | Ubuntu 22.04 容器，编一次两种包共用 | 只依赖 libc / libm / libgcc；在 glibc 2.35 上编，实际门槛是 `GLIBC_2.34`。工具链与 crate 缓存直接挂本机的 `~/.cargo` / `~/.rustup`；依赖树里有 `openssl-sys`，编译要 `libssl-dev`（最终二进制并不链 libssl） |
| fcitx5 插件（deb） | Debian 13 容器 | 插件要对着目标发行版自己的 fcitx5 编；`CMakeLists.txt` 要 5.1.8+，Ubuntu 24.04 只有 5.1.7 |
| fcitx5 插件（rpm） | Fedora 42 容器 | 同上 |
| 随包资源 | 本机，`files.py resources` | 与 `install.sh` 同一套产品数据校验（`tools/release/data.lock`）与布局，含本地整句模型 |

`files.py` 收产品数据时跳过点开头的文件：data-v1 发布包里混着 19 个 macOS 的 AppleDouble（`._law.qj` 这种），
不跳的话会被当成领域词库装进 `dicts/`（`install.sh` 以前也会装进去）。

## 包里装了什么

deb 与 rpm 都拆成两个包：

- **`qingjian`**：`/usr/bin/qingjian-linux-server`、`/usr/share/qingjian/resources/`（词库、释义表、本地整句模型；
  Server 按「可执行文件上一层的 `share/qingjian/resources`」找到它）、`/usr/lib/systemd/user/qingjian-server.service`
  （`packaging/qingjian-server.service`）。装的时候 `systemctl --global enable`，每个用户登录后自动起 Server，
  并对已经登录的用户当场起起来（见文末）；卸载时 disable 并停掉。
- **`qingjian-fcitx5`**：插件与两份描述文件，放 fcitx5 自己的插件目录（Debian 是 `/usr/lib/<多架构>/fcitx5/`，Fedora 是 `/usr/lib64/fcitx5/`），
  描述文件里 `Library=qingjian` 按名字找。

分成两个包，是因为插件要对着各发行版的 fcitx5 编、依赖也不同；以后加别的前端（IBus）只多一个包，Server 与数据那个不动。
动态库依赖都从二进制推（deb 用 `dpkg-shlibdeps`，rpm 自动生成），不手写：`qingjian` 只要 `libc6 (>= 2.34)`，
`qingjian-fcitx5` 要 `libfcitx5core7 (>= 5.1.12)`。

## 装法

Server 装完就在已登录用户的会话里起来了，不用重新登录。重启 fcitx5，在 `fcitx5-configtool` 里取消「仅显示当前语言」、添加「青简」。

```bash
# Debian 13 / Ubuntu 25.04 及以后
sudo apt install ./qingjian_<版本>_amd64.deb ./qingjian-fcitx5_<版本>_amd64.deb
# Fedora 42 及以后
sudo dnf install ./qingjian-<版本>.x86_64.rpm ./qingjian-fcitx5-<版本>.x86_64.rpm
# openSUSE Tumbleweed（包没签名）
sudo zypper install --allow-unsigned-rpm ./qingjian-<版本>.x86_64.rpm ./qingjian-fcitx5-<版本>.x86_64.rpm
```

`apt install ./…` / `dnf install ./…` 会把 fcitx5 与它的 GTK / Qt 输入法模块、配置工具一起装上（2026-09-23 在干净的
Debian 13、Ubuntu 26.04、Fedora 42 容器里看过；`dpkg -i` 不补依赖，要再跑一次 `sudo apt -f install`）。
**不会替你把输入法框架从 IBus 切到 fcitx5**，GNOME 默认是 IBus：

- Debian / Ubuntu：`im-config -n fcitx5`，重新登录；
- Fedora：装 `fcitx5-autostart`（`qingjian-fcitx5` 已经 Recommends 它，dnf 缺省会一起装），重新登录。

用 `install.sh` 装过的先跑 `apps/linux/scripts/uninstall.sh`：它把插件描述文件装在 `~/.local/share/fcitx5/`，同名时用户目录那份优先，装的包就不生效。

排错：`systemctl --user status qingjian-server`、`journalctl --user -u qingjian-server`；
`python3 apps/linux/packaging/smoke/check.py nihao` 不经过 fcitx5 直接问 Server 要候选。

## 怎么验

```bash
apps/linux/packaging/smoke/run.sh
```

把 `target/linux-pkg/out/` 的包拿到 Debian 13、Ubuntu 26.04、Fedora 42、最新 Fedora、openSUSE Tumbleweed 的干净容器里：
装两个包，看 systemd 自启登记、插件缺不缺库，起 Server 用 `check.py` 打 `nihao`，等本地整句模型加载完，卸载后看自启登记清掉没有。
2026-09-23 五个都过：候选「你好 👋 你好好 你好像…」，模型从 `/usr/share/qingjian/resources/data/model/model.qjm` 加载并预热。

容器里没有图形会话，**真会话里在 fcitx5 下切到青简打字没验**：要在机器上装了之后手测。

## 坑：`systemctl --global enable` 不够

它只对之后起来的用户 systemd 实例生效，而注销再登录得快时实例根本不重起。2026-09-23 在 Ubuntu 24.04 上踩过：
10:51 装包、10:52 重新登录，用户实例是 10:46 起的一直没换，Server 一次都没起。所以 postinst / `%post` 对
`loginctl list-users` 里的每个用户 `systemctl --user -M 用户@ daemon-reload` 与 `restart`（升级时也换上新的 Server），
prerm / `%preun` 同样 stop。同一台机器上升级验过：装包那一秒 Server 就换成了新版，没有重新登录。

- **`--global` 对登录界面也生效**：gdm 这类系统账户也有自己的用户会话（`loginctl list-users` 里能看到 `gdm`），
  不拦的话 greeter 里也会起一个 Server、白白加载词库与模型。服务单元写 `ConditionUser=!@system` 跳过系统账户（systemd 237+）。
  装包脚本取用户名用 shell 的 `read`，不靠 awk（openSUSE 最小容器里没有 awk）。
