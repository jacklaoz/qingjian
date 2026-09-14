# qingjian-linux

青简的 Linux 输入法壳。**只做 Wayland，不做 X11**——方案取舍、分期与验收在
[docs/plan/linux_plan.md](../../docs/plan/linux_plan.md)。

现在是 **L0 骨架**：路径定位、按键翻译、自检就位，还没接输入法框架，所以装上也还不能打字。

```bash
cargo run -p qingjian-linux -- --check     # 找一遍用户目录、随包数据与配置，打印缺什么
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
