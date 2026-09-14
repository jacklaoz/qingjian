# qingjian-linux

青简的 Linux 输入法壳。走 IBus（方案 C），**X11 与 Wayland 都能用**——
计划里的「只做 Wayland」说的是将来那一步自绘候选窗只做 wlroots 的 `input-method-v2`，不为 X11 单写窗口。
方案取舍、分期与验收在
[docs/plan/linux_plan.md](../../docs/plan/linux_plan.md)。

现在到 **L1**：走 IBus 接上了（方案 C），敲拼音出候选、选词上屏、配置热加载都通了。
还没做 wlroots 系的原生 `input-method-v2` 自绘（L4），也还没打包（L2）。

```bash
cargo run -p qingjian-linux -- --check           # 找一遍用户目录、随包数据与配置，打印缺什么
cargo run -p qingjian-linux -- --ibus-xml        # 打印 IBus 组件 XML
cargo run -p qingjian-linux -- --ibus            # 作为 IBus 引擎跑（通常由 ibus-daemon 拉起）
```

## 怎么在本机验

`apps/linux/tests/ibus_engine.rs` 是对着**真的 ibus-daemon** 跑的端到端测试：建输入上下文、切成青简、
敲 `nihao`、断言候选表里有「你好」、空格上屏。本机没有 ibus 时它直接跳过（CI 就是这种情况）。

起一个不碰当前会话的私有 daemon 来跑它：

```bash
cargo build -p qingjian-linux
mkdir -p /tmp/qj/component
cargo run -q -p qingjian-linux -- --ibus-xml "$PWD/target/debug/qingjian-linux" > /tmp/qj/component/qingjian.xml
env IBUS_COMPONENT_PATH=/tmp/qj/component ibus-daemon -r -d -s --panel disable --config disable -t refresh
cargo test -p qingjian-linux --test ibus_engine -- --nocapture
```

这个测试只断言到「引擎走了非空 preedit 那条分支」：带内容的 `UpdatePreeditText` 不会转发到合成客户端上
（起不起面板都一样）——真实应用的 preedit 由 GTK / Qt 的输入法模块自己渲染，不走「客户端订阅 InputContext 信号」这条路。

**真机验过**（2026-09-14，GNOME / Wayland / gnome-text-editor）：`nihao` → 你好、`keyi` → 可以、
`cece` → 的的（整句）都对，拼音行与候选窗正常，学习六张表照常落盘。
在**不动当前输入法**的前提下这么验：起私有 daemon（上面那几条）后把全局引擎切成青简，
再单独给一个应用指过去——其他程序照旧用系统自己的输入法。

```bash
ADDR=$(ibus address)
gdbus call --address "$ADDR" --dest org.freedesktop.IBus --object-path /org/freedesktop/IBus \
  --method org.freedesktop.IBus.SetGlobalEngine qingjian
env GTK_IM_MODULE=ibus IBUS_ADDRESS="$ADDR" gnome-text-editor
```

## 打包

```bash
apps/linux/packaging/build-deb.sh              # 三个 .deb 出在 target/deb/
apps/linux/packaging/build-deb.sh --skip-data --skip-model   # 只打程序包
```

分三个包：`qingjian`（程序，2 MB）、`qingjian-data`（词库 / 语言模型 / 释义表，27 MB）、
`qingjian-model`（本地整句模型，49 MB）。发行版不收一百多兆的单包，而且数据与模型的更新节奏跟代码不一样。
程序包不依赖后两个：缺语言模型退化成一元词频、缺模型不重排，Core 本来就支持。

数据来自 `data` 预发布（`docs/notes/release.md`），仓库里没有，打数据包前要先取：

```bash
BASE=https://github.com/qingjian-team/qingjian/releases/download/data
mkdir -p data/generated data/model
curl -sL "$BASE/qingjian-data.tar.gz" | tar xz -C data/generated
curl -sL -o data/model/model.qjm "$BASE/model.qjm"
```

## 目录怎么分

只读的随包数据走 `qingjian_platform::resources`，按「与可执行文件同级 → 仓库开发布局 → `/usr/share/qingjian`」找；
用户数据按 XDG 分三处，而不是像 macOS 壳那样全塞在一个目录里：

| 东西 | 位置 |
|---|---|
| 配置 `config.toml` | `$XDG_CONFIG_HOME/qingjian`（缺省 `~/.config/qingjian`） |
| 学习数据、输入日志、统计、词汇记录、导入的词库 | `$XDG_DATA_HOME/qingjian`（缺省 `~/.local/share/qingjian`） |
| 日志 | `$XDG_STATE_HOME/qingjian`（缺省 `~/.local/state/qingjian`） |

环境变量填的是相对路径时按 XDG 规定当作没设，退回 `$HOME`，免得在当前工作目录下乱建目录。

## 按键

`keys` 模块只认 X11 keysym，两条接入路线翻出来的都是它，所以这一层不认 IBus 也不认 Wayland：

- 方案 C（IBus）：`ProcessKeyEvent` 直接给 keysym 与修饰键掩码。
- 方案 A（原生 Wayland）：键盘抓取给 keycode，xkbcommon 翻成同一套 keysym。

修饰键复用协议里的 `KeyModifiers`（ctrl / shift / alt / win / caps / english_mode 六个位本来就平台无关）；
按键本身**不复用**协议的 `KeyEvent`——那个的 `virtual_key` 是 Windows 的 VK 码，取值空间与 keysym 不一样，混用迟早出错。

## 这个平台上做不到的事

`[apps]` 按应用配置在 Linux 上永远不命中：它在 macOS 靠 bundle identifier、Windows 靠 exe 文件名，
而 Wayland 客户端拿不到前台应用标识（X11 的 `WM_CLASS` 那条路随 X11 一起排除了）。
配置项仍然读得进、不报错，缺省名单是空的，配置模板里也写明了。
终端 / 编辑器里不想要英文候选，用 `[general] english_candidates = false` 整个关掉。
