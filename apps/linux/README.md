# 青简 Linux 输入法

Linux 端是**一个产品、两个产物**，各自一个 package，同放本目录：

| 目录 | package | 产物 | 职责 |
| --- | --- | --- | --- |
| `ime/` | `qingjian-linux` | `qingjian-linux` | 输入法本体：走 IBus 接上 `qingjian-core::Engine`，把候选交给 IBus 面板画 |
| `settings/` | `qingjian-linux-settings` | `qingjian-settings` | GTK4 设置界面：左侧导航 + 各分节表单，读写 `~/.config/qingjian/config.toml` |
| `packaging/` | — | 四个 `.deb` | `build-deb.sh`：程序 / 设置界面 / 数据 / 模型 |

接入方案（IBus 而不是 Fcitx5 或原生 `input-method-v2`）与分期见 [`docs/plan/linux_plan.md`](../../docs/plan/linux_plan.md)，
搭起来时踩的坑与验收数字在 [`docs/notes/linux-bringup.md`](../../docs/notes/linux-bringup.md)。

## 分层

```text
ibus/        D-Bus 那层：GVariant 序列化、工厂、引擎对象、总线地址、当前聚焦的对象
  ↑
dispatch/    按键 → 帧。不认 IBus 也不认 Wayland，所以能脱离输入法框架整段测
  ↑
assembly/    装 Engine（与 macOS 的 host/init.rs、Windows 的 assembly/ 是同一件事的第三份）
  ↑
Core
```

绝大多数测试在 `dispatch` 那一层：不用起 daemon、不用 D-Bus。**这一层里不许出现排序、词库、翻译或文本变换**——
换掉 IBus 换成别的框架，不该需要改 Core 的任何一行（见 [`docs/contributing.md`](../../docs/contributing.md)）。

## 构建与自检

```bash
cargo build --release -p qingjian-linux -p qingjian-linux-settings
cargo test -p qingjian-linux                      # 不需要 ibus，端到端那两条会自己跳过
target/release/qingjian-linux --check             # 用户目录、随包数据、本地整句模型、配置各找一遍
```

`--check` 是装机后第一件该跑的事：它只定位文件、不装配 Engine，所以数据没就位时也能跑完并说清楚缺什么。
**随包数据靠可执行文件的位置找**（exe 旁 → 仓库开发布局 → `/usr/share/qingjian`），
所以从 `target/release/` 跑到的是仓库里的 `data/generated/`，装到 `/usr/bin` 跑到的是 `qingjian-data` 包装的那份。

## 对着真 ibus-daemon 跑端到端

`ime/tests/ibus_engine.rs` 是唯一能验证「IBus 那套 GVariant 对象拼对了没有」的办法——签名错一位不报错，
只是候选窗静默不出现。它需要一个真的 ibus-daemon，**但别用桌面会话那一条**：那会把当前输入法切成青简、
把测试打的字写进你真实的学习数据。起一条私有的：

```bash
cargo build --release -p qingjian-linux

export QJ=$(mktemp -d)
mkdir -p "$QJ/component" "$QJ/config" "$QJ/data" "$QJ/cache"
target/release/qingjian-linux --ibus-xml "$PWD/target/release/qingjian-linux" \
    > "$QJ/component/qingjian.xml"

env XDG_CONFIG_HOME="$QJ/config" XDG_DATA_HOME="$QJ/data" XDG_CACHE_HOME="$QJ/cache" \
    IBUS_COMPONENT_PATH="$QJ/component" \
    ibus-daemon -a "unix:path=$QJ/bus" -p disable -c disable -t refresh -d
QJ_DAEMON=$(pgrep -f "unix:path=$QJ/bus")

# 按键 → preedit → 候选表 → 上屏
QINGJIAN_IBUS_TEST=1 IBUS_ADDRESS="unix:path=$QJ/bus" \
    cargo test --release -p qingjian-linux --test ibus_engine

# 停手之后主循环自己推的那一帧（本地整句模型重排完）；要 data/model/model.qjm
QINGJIAN_IBUS_TEST=1 IBUS_ADDRESS="unix:path=$QJ/bus" \
    cargo test --release -p qingjian-linux --test ibus_engine -- --ignored --test-threads=1

kill "$QJ_DAEMON" && rm -rf "$QJ"
```

三处不这么写就会浪费一下午：

- **`IBUS_COMPONENT_PATH` 才是决定组件从哪读的**，`XDG_DATA_DIRS` 不管用（`-t refresh` 也不管用）。
  机器上装过 `.deb` 时，`/usr/share/ibus/component/qingjian.xml` 与开发版**同名**（`org.freedesktop.IBus.Qingjian`），
  daemon 挑的是系统那份，症状是「改了代码重新编译，行为一点没变」。
  `pgrep -af 'qingjian-linux --ibus'` 打出来的路径就是 daemon 实际拉起的那个，一眼能看出是 `/usr/bin` 还是 `target/release`。
- **四个 XDG 变量都要设**：私有 daemon 的地址文件写在 `$XDG_CONFIG_HOME/ibus/bus/` 下，不隔离就会盖掉桌面会话那份，
  新起的应用会连到私有 daemon 上去。`XDG_DATA_HOME` 隔离的是青简自己的学习数据。
- **两条端到端不能并发**：一个进程里只有一份 Router（IBus 一个引擎实例服务当前焦点），
  所以第二条标了 `#[ignore]`，要单独用 `--ignored --test-threads=1` 跑。

引擎进程是 daemon 拉起来的，它的 stderr 被 daemon 丢掉，看不到日志。要看就把 `--ibus-xml` 指向一个包装脚本：

```bash
cat > "$QJ/engine.sh" <<EOF
#!/bin/sh
exec $PWD/target/release/qingjian-linux --ibus 2>> $QJ/engine.log
EOF
chmod +x "$QJ/engine.sh"
target/release/qingjian-linux --ibus-xml "$QJ/engine.sh" \
    | sed 's| --ibus</exec>|</exec>|' > "$QJ/component/qingjian.xml"
```

再起 daemon 时带上 `RUST_LOG=qingjian_linux=debug`，模型加载、重排、热加载都会记在 `$QJ/engine.log` 里。

## 装到本机

```bash
apps/linux/packaging/build-deb.sh          # target/deb/ 下四个包；--skip-data / --skip-model 可跳过大包
sudo apt install ./target/deb/*.deb        # 用 apt 不用 dpkg -i：Recommends 里的 ibus-gtk4 / 中文字体才会一起装
ibus restart                               # 或注销重登，否则输入源列表里看不到
```

包分四个、依赖怎么声明（尤其是**手写 control 必须自己算 `libc6` 那条**）见 `packaging/build-deb.sh` 的文件头
与 [`docs/notes/linux-bringup.md`](../../docs/notes/linux-bringup.md)「打包与安装」。

装好之后：

- 输入法本体 `/usr/bin/qingjian-linux`，随包数据 `/usr/share/qingjian/{data,assets}`，组件 XML `/usr/share/ibus/component/qingjian.xml`。
- 配置 `~/.config/qingjian/config.toml`（改了即时生效，主循环每秒看一次 mtime），用户数据 `~/.local/share/qingjian/`，日志 `~/.local/state/qingjian/`。
- **云服务密钥读 `~/.config/qingjian/.env`**（`QINGJIAN_API_KEY=<密钥>` 一行）或配置里的 `[predict] api_key`。
  引擎进程是 ibus-daemon 拉起来的，**终端里 export 的变量它看不到**；开发时起私有 daemon 那条命令上带的环境变量它倒是继承得到。
- **候选窗与拼音行是 IBus 面板画的**，字体配色跟着桌面走；`[general]` 里的 `layout` / `theme` 在这条路上不生效（自绘是 L4 的事，只做 wlroots）。

用户视角的安装、桌面差异与排查在 [`docs/user/getting-started/install.md`](../../docs/user/getting-started/install.md)。

## 版本与发布

两个 package 的 `version` 各自写在自己的 `Cargo.toml`（`apps/*` 各壳独立发布，不跟 workspace 走），
`.deb` 的版本号从 `ime/Cargo.toml` 读，`-dev` 落成 Debian 认的 `~dev`。

**Linux 还没有发版流程**：`release.yml` 只认 `macos-v*` / `windows-v*` 两种标签，`.rpm` 也还没有
（没有能验的环境，不写没跑过的打包脚本）。现在给测试者的包是本机跑 `build-deb.sh` 出的。
