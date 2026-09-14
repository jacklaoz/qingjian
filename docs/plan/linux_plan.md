# Linux 支持计划

Phase 5 的后半段（前半是 Windows TSF）。2026-09-14 定的路线与分期，实施前这份文件是唯一依据；
动手后实现与这里分歧，以代码为准并回来改这份文件。

**只做 Wayland，不做 X11。** 这条决定贯穿全文，直接决定了候选窗能不能自绘（见下面前提 ③）。

Core 已经是平台无关的，Linux 要做的全部是壳：把系统输入事件翻译成 `Engine` 的输入，把 `Engine` 返回的候选画出来。
判断标准与 macOS / Windows 一致：**这份计划里不应该出现任何排序、词库、翻译或文本变换的改动**。

## 一、Linux 与前两个平台不一样的三件事

**① 进程模型比 Windows 简单。** TSF DLL 会被注入每个应用进程，所以 Windows 必须把 Engine 放进独立 Server 进程、
两边走命名管道。Linux 不存在这个问题：IBus engine 是 ibus-daemon 按需拉起的**独立进程**，Fcitx5 addon 加载进
**fcitx5 单进程**，两者都不进应用进程。所以 Engine 可以进程内单例，跟 macOS 一样，**不需要 IPC**。
`qingjian-platform::protocol` 那套 `ClientMessage` / `ServerMessage` 在 Linux 上用不着；
值得照搬的只有 `Frame` 这个**数据模型**（preedit 分段 + 候选页 + 高亮 + 页码 + 整句补全 + 提示）。

**② Engine 要串行访问。** `Engine` 有 5 个 `RefCell` / `Cell` 缓存（`span_cache`、`last_query`、
`correction_cache`、`neural_cache`、`last_rescored`），所以不是 `Sync`；注入的 trait（`Translator` / `Learner` /
`Predictor` / `SentenceScorer`）都要求 `Send`。**它是 `Send` 的**（2026-09-14 实测，`dispatch::tests` 里有一条
编译期断言钉着），所以套一层 `Mutex` 就满足 zbus 对接口对象 `Send + Sync` 的要求，不必像原先设想的那样
另开一条工人线程来持有它。`Router` 因此可以直接 `Arc<Mutex<Router>>` 交给 D-Bus 那层。

**③ 只做 Wayland，不做 X11**（2026-09-14 定）。这把最大的约束推到了台前：**Wayland 客户端不能给自己的窗口绝对定位**，
所以候选窗摆不到光标处这件事没有 X11 那种 override-redirect 的绕法。输入法 popup 在 Wayland 上唯一的正路是
`input-method-unstable-v2` 的 `zwp_input_popup_surface_v2`——由合成器按文本光标替你摆位——而**只有 wlroots 系
（sway / Hyprland / river / niri）实现它，GNOME 与 KDE 都不对第三方开放**。

由此得到一条贯穿全文的分界，它不是过渡期状态而是终态：

- **GNOME / KDE 的 Wayland 会话**：只能把候选交给 IM 框架自带的面板，**永远画不了自己的候选窗**。
- **wlroots 系**：可以当 `input-method-v2` 客户端自绘 popup，视觉与 mac / Windows 齐平。

## 二、三个方案

### 方案 A：原生 Wayland `input-method-v2` 客户端 + 自绘 popup

不经 IBus 也不经 Fcitx5，直接连合成器：`wayland-client` 绑 `zwp_input_method_manager_v2`，
按键来自 `zwp_input_method_keyboard_grab_v2`（xkbcommon 把 keycode 翻成 keysym），
preedit 走 `set_preedit_string`、上屏走 `commit_string`；候选窗是 `zwp_input_popup_surface_v2`——
**合成器按文本光标替你摆位，客户端只管画内容**，正好绕开「客户端不能定位窗口」。

视觉与 mac / Windows 完全一致：字号分级、生词橙色、假名淡色、云朵标记、preedit 右侧的整句补全、
删候选提示，全都画得出来。符合「显示面自绘、控件面原生」这条约束。绘制不必新写：
`crates/qingjian-render` 出的预乘 RGBA 位图直接进 `wl_shm` 缓冲区。

代价：**只覆盖 wlroots 系**（sway / Hyprland / river / niri）。GNOME 与 KDE 的 Wayland 会话拿不到
`input-method-v2`，那里只能走方案 C。所以 A 不是 C 的替代品，是 C 之上给 wlroots 用户的增强。

### 方案 B：Fcitx5 addon（C++ shim + Rust cdylib）

`fcitx5` 进程加载 `qingjian.so`（C++ addon），它 `dlopen` 一个导出 C ABI 的 Rust cdylib。
分层思路与 Windows 的「薄 DLL + 厚 Server」一致，只是同进程 FFI 而不是管道。

候选、preedit、**定位全部由 fcitx5 的 panel 负责**——GNOME（im-module）/ KDE（原生）/ wlroots 都已经有人踩平，
定位问题一笔勾销，而且是**三类桌面里唯一一个又能自绘之外全覆盖、又不必自己处理摆位**的方案。
`CandidateWord` 有 `comment` 字段可以放译文。中文用户在 Linux 上装 fcitx5 的比 ibus 多（Rime / 小鹤 / 搜狗都在 fcitx5）。

代价：引入 C++（本项目目前零 C++）；视觉受 panel 主题限制，**字号分级、生词橙色、假名注音很可能做不出**，
等于放弃「显示面自绘」；fcitx5 无官方 Rust 绑定，C ABI 要自己定并自己维护。

### 方案 C：IBus engine（zbus）+ 系统面板

纯 zbus，候选交给 IBus 自带的 lookup table，译文塞进同一个 `IBusText` 用前景色 attribute 区分。

**GNOME / KDE / wlroots 的 Wayland 会话立刻全覆盖**（面板自己管定位），零窗口代码，纯 Rust。
这是唯一能覆盖 GNOME 与 KDE 的路径，所以它不是临时方案，而是这两个桌面上的**终态**。

代价：`IBusAttribute` 只有前景色 / 背景色 / 下划线，**没有字号**，视觉层级塌成一层；
生词橙色能做（前景色），词性与假名注音只能同字号；整句补全与删候选提示没地方画，要挪进 aux text。

## 三、选定路线：C → A，中间合入 renderer-spike

**先用 C 把 Linux 打通，再按 A 补自绘。** 理由：

1. **C 覆盖 GNOME 与 KDE，A 覆盖不了**。按桌面份额算，C 是主路径、A 是增强，顺序不能倒。
   而且 Linux 上真正未知的东西（XDG 路径、keysym 映射、`.deb` / `.rpm` 打包、一百多兆随包数据怎么分发）
   和渲染完全正交，C 能在一两周内把它们全趟一遍，趟出来的对 A 和 B 都有效，不会白做。
2. **自绘那一步不需要从零写。** `crates/qingjian-render`（分支 `renderer-spike`，见
   [notes/crate-notes.md](../notes/crate-notes.md)）已经是「候选窗一帧 + 主题 → 预乘 RGBA 位图」，
   tiny-skia 栅格 + cosmic-text 文字，mac 壳 `candidates/bitmap/` 已经在贴它的位图。
   **预乘 RGBA 位图正好是 `wl_shm` 缓冲区要的东西**，A 只需要绑协议 + 贴图，摆位由合成器负责，
   不必再写第三份绘制代码。所以 A 的前置是把这个分支合进 main，而不是新立一个 crate。
3. **B 是 A 与 C 之间的中间档，保留为逃生舱。** A 好看但只有 wlroots，C 全覆盖但视觉塌成一层；
   B 是「GNOME / KDE 也覆盖得到、视觉又比 IBus 面板好一档」的唯一选项。
   如果 L1 之后发现 C 的视觉降级严重到伤产品（译文那条 annotation 本来就是青简的卖点），
   就把 B 提上来替掉 C 在 GNOME / KDE 上的位置；那时 L0–L2 的基础设施仍然全部有效，换掉的只有最上面那层壳。

顺带要解决的一处重复：Windows `apps/windows/server/src/dispatch/key/input.rs` 的文件头写着
「分流规则与 macOS 壳的 `handle_text` / `handle_command` 对齐」——那是手工对齐的孪生逻辑。
Linux 是第三份。按键分流是平台无关的（它只认字符、功能键语义和配置里的快捷键），
应该在 L3 抽成共用 crate，Linux 直接复用，否则三份手工对齐必然走偏。

## 四、分期

| 阶段 | 内容 | 估时 |
|---|---|---|
| **L0 打地基**（2026-09-14 做完） | `apps/linux/` 骨架；XDG 路径（配置 `~/.config/qingjian/`、数据 `~/.local/share/qingjian/`、日志 `~/.local/state/qingjian/`）；`qingjian-platform` 补 Linux 臂（`config/apps.rs` 与 `config/shortcut.rs` 现有 `cfg(windows)` 分支、`resources.rs` 加 `/usr/share/qingjian`）；xkbcommon keysym → Core 输入的映射表（A 与 C 共用） | 2–3 天 |
| **L1 方案 C 跑通** | zbus 实现 IBus engine 接口；`ProcessKeyEvent` 接 Core（分流规则对齐 macOS `handle_text` / `handle_command`）；preedit + lookup table + commit；Engine 钉单线程；`config.toml` 热加载 | 4–6 天 |
| **L2 能日用**（2026-09-14 做完） | 轮询节拍、每 60 秒落盘、边界 `catch_unwind`、`.deb`（程序 / 数据 / 模型三个包）都已做；`.rpm` 还没有（没有能验的环境） | — |
| ↑ **到这里可发 Linux alpha** | | **≈ 2 周** |
| **L3 合入渲染层** | `renderer-spike` 合进 main（连同 `docs/design/rendering.md`）；按键分流抽成共用 crate 供三端复用 | 见分支现状 |
| **L4 方案 A 自绘（仅 wlroots）** | `wayland-client` 绑 `zwp_input_method_manager_v2` + `keyboard_grab`；`zwp_input_popup_surface_v2` 贴 `qingjian-render` 的位图到 `wl_shm`；启动时探测合成器有没有 `input-method-v2`，没有就落回 C 的 IBus 路径 | 5–8 天 |
| **L5 收尾** | 状态条；GTK4 设置程序（L1–L4 期间只给注释详尽的 `config.toml` 模板，它本来就热加载） | 按需 |

## 五、已知风险与未决问题

- **`[apps]` 按应用配置在 Linux 上直接不可用。** 它在 macOS 靠 bundle identifier、Windows 靠 exe 文件名；
  Wayland 客户端**拿不到前台应用标识**，而 X11 的 `_NET_ACTIVE_WINDOW` / `WM_CLASS` 这条路已经排除（前提 ③）。
  结论：`[apps] english_candidates_off` 这类配置在 Linux 上读得进、但永远不命中。
  配置项要优雅降级（读到不报错、设置界面里标注「本平台不支持」），不要为它留半截实现。
- **随包数据体积。** `dict.qj` 3 MB + 领域词库 7 MB + `lm.qj` 29 MB + 释义表 39 MB + `model.qjm` 56 MB ≈ 134 MB。
  发行版打包不接受这个体积的单包，必须拆成 `qingjian` / `qingjian-data` / `qingjian-model` 三个包，
  `resources.rs` 的定位逻辑要能容忍后两个缺席（缺语言模型退化成一元、缺模型不重排，Core 本来就支持）。
- **设置界面是第三套。** mac 是 AppKit、Windows 是 WinUI 3。Linux 前期不做，推到 L5 之后再看是否值得写 GTK4。
- **`qingjian-neural` 在 Linux 上走 CPU**（与 Windows 同一条路径），代码零改动，但加载与每次重排的耗时要真机量；
  超了就缩前文长度，同 Windows 的处理。
- **发行渠道未定。** `.deb` / `.rpm` 是底线；AUR 值得做；Flatpak 对输入法不友好（需要 host 的 IM 总线），先不碰。
- **合成器探测。** 同一台机器上用户可能在 GNOME 与 sway 之间切换会话，A 与 C 要能在启动时按
  「合成器暴露不暴露 `input-method-v2`」自动选路，而不是靠用户改配置。这条要在 L4 之前就把开关留好。
- **待验证的假设**：IBus 的 `IBusAttribute` 到底能不能在 lookup table 的候选文本里做到「译文换色」而不影响候选词本身；
  做不到的话 L1 的译文只能进 aux text，产品形态要重新看。这是 L1 第一天就该验的事。

## 六、实施记录

### L2（2026-09-14，做完；`.rpm` 除外）

`apps/linux/packaging/build-deb.sh` 打三个包：程序 2 MB、数据 27 MB、模型 49 MB。
装到 `/usr/bin/qingjian-linux` + `/usr/share/qingjian/{data,assets}`，组件 XML 落 `/usr/share/ibus/component/`。
同日在本机装了一遍并切成了会话输入法（im-config 从 fcitx5 改 ibus）。

装完第一次自检就炸出一个**静默降级**的 bug，单测与开发机上都碰不到：
`resources::bundled_root` 的开发布局回退是「exe 往上数三层」，装到 `/usr/bin` 之后往上三层正好是 `/`，
而这台机器根目录下有个 `/data`，于是 `/` 被当成随包根，回落到几十条的样例词库——
症状是「装好了、能启动、就是一个词都打不出」。改成先确认 exe 真在 `target/{debug,release}/` 里才认开发布局，
补了三条测试钉住。这条 macOS 走自己的 `paths.rs` 不受影响，**Windows 与 Linux 共用这个函数**。

#### 装机后的验收数字（2026-09-14，GNOME / Wayland，真产品数据）

装完切成会话输入法日常打了一阵，`ibus` + 三个 `.deb`（程序 / 数据 / 模型）：

- **每键耗时**：`qingjian-cli --typing zhonghuarenmingongheguowansui`（release）29 键，
  **最慢 3.34 ms、平均 857 µs**，项目标准是 10 ms 以内 —— 达标，且与 macOS 同量级。
  整句转换一口气出「中华人民共和国万岁」。
- **引擎常驻内存** 约 104 MB（词库 mmap + 42 MB 语言模型 + 释义表）。
- **学习链路**：`user.tsv` / `user-ngram.tsv` / `user-choices.tsv` / `user-english.tsv` / `usage.tsv`
  都在按预期写，中英混输（`clone` 原样直通）与整句、词级都走通。

### L2 前半（2026-09-14，做完）

轮询节拍、每 60 秒落盘、配置热加载随 L1 一起做了。另外补了边界的 panic 隔离（`ibus/shared.rs`）：
按 `docs/design/architecture.md`「崩溃不丢」那条，拦下后把组句清干净、这个键让给应用，输入法接着服务。
**顺带修掉一个把整个输入法拖死的写法**：原先每处都是 `lock().expect(...)`，而 `Mutex` 一旦被 panic 毒化，
之后每次 `lock()` 都返回错误——等于「崩过一次就再也打不了字」。现在统一走 `Shared::lock`，毒化了就把状态取回来接着用。

剩下的是打包（`.deb` / `.rpm`）与一百多兆随包数据怎么分三个包，那部分要在装机环境里验，没法在开发机上跑完。

### L1（2026-09-14，做完）

IBus 接上了：敲拼音出候选、数字 / 空格选词上屏、翻页、退格、Esc、中英模式、`config.toml` 热加载都通。
`apps/linux` 现在是 `assembly`（装 Engine）+ `dispatch`（按键 → 帧，不认 IBus）+ `ibus`（D-Bus 那层）三段。
80 个单元测试 + 1 个对着真 ibus-daemon 跑的端到端测试（没有 ibus 就跳过，所以 CI 上不会红）。

真机跑出来才看清的四件事：

- **ibus-daemon 不给引擎进程设 `IBUS_ADDRESS`**，得自己读地址文件。而且**不能扫目录随便挑**：
  `~/.config/ibus/bus/` 里常年躺着别的输入法或上次会话留下的陈旧文件（这台机器上就有 fcitx 留的
  `-unix-0`，按字典序还排在当前会话的 `-unix-wayland-0` 前面），挑中死的那个连上去一个信号都收不到。
  改成按 ibus 自己的规矩算文件名（`<机器 id>-<主机>-<显示标识>`），算不出来才退回扫目录且只认 PID 还活着的。
- **Shift + 数字在 X11 上 keysym 是 `!` 不是 `1`**，而缺省的删候选快捷键正是 Shift + 数字。
  只看字符这条快捷键在 Linux 上会整个失效，必须同时用硬件键码按**键位**认数字（Windows 用虚拟键码干同一件事）。
- **IBus 的 GVariant 对象嵌套要包变体**：`IBusText` 的第四项签名是 `v`，直接塞结构体会内联成
  `(sa{sv}av)`，签名错了不报错、只是静默不显示。签名现在由单元测试钉死。
- **合成客户端收不到带内容的 `UpdatePreeditText`**（起不起面板都一样），一度以为是我们拼错了。
  真相是真实应用的 preedit 由 GTK / Qt 的输入法模块自己渲染，不走「客户端订阅 InputContext 信号」这条路，
  所以拿 D-Bus 客户端当「应用」来验 preedit 本来就验不到。同日在 gnome-text-editor 里真机验过：
  `nihao` → 你好、`keyi` → 可以、`cece` → 的的（整句），拼音行、候选窗、上屏、学习落盘全都对。
  **教训**：合成客户端能验协议对不对，验不了「用户看到什么」，后者只能开一个真应用。

### L0（2026-09-14，做完）

`apps/linux` 进 workspace，拆成 lib + bin（与 Windows Server 同形状）。`cargo run -p qingjian-linux -- --check` 能跑，
fmt / clippy / 全量测试（CI 的 `--workspace --exclude qingjian-macos`）全绿，新增 15 个测试。

平台层改了三处 Linux 臂：`config/apps.rs` 的缺省名单、`config/mod.rs` 的两个配置模板宏、`resources.rs` 的系统布局。
原来的 `cfg(not(windows))` 把 Linux 一并算进 macOS，缺省名单会给 Linux 发一份 bundle identifier；
现在门控改成 `cfg(target_os = "macos")` 与 `cfg(not(any(windows, target_os = "macos")))`，
`default_list_follows_the_platform` 这个测试也跟着改了断言。

两条动手后才看清的事：

- **`[apps]` 按应用配置在 Linux 上确定做不了**，不是「Wayland 上大概率要放弃」那种程度。缺省名单直接定成空的
  （`DEFAULT_ENGLISH_CANDIDATES_OFF_LINUX`），配置模板里写明原因并指向 `[general] english_candidates`。
- **Engine 装配（Windows `assembly/`、macOS `host/init.rs`）不能提到 `qingjian-platform`。** 它本身是平台无关的，
  看着正是共用的料，但 TSF DLL 依赖 platform crate，而架构约束写着「DLL 不能带 Engine 的依赖树」；
  把 learning / translate / lm 的依赖加进 platform 就等于给 DLL 挂上整棵树。它得等 L3 那个新 crate，
  所以 L0 不碰装配，Engine 装配推到 L1 各写各的、L3 再收口。

## 七、验收

与前两个平台同一把尺子，不新立标准：

- `cargo clippy --all-targets -- -D warnings` 与全 workspace 测试在 Linux 上绿。
- 每键耗时用 `qingjian-cli --typing` 的 release 构建量，目标仍是 10 ms 以内（Core 没改，这里量的是壳有没有引入额外开销）。
- 端到端在两类会话上各走一遍（GNOME 或 KDE 的 Wayland 会话走 C，sway 或 Hyprland 走 A）：
  整句、简拼、双拼、注音、模糊音、纠错、英文模式、emoji、快捷候选、翻页、删候选、译词上屏，
  行为与 [user/getting-started/keys.md](../user/getting-started/keys.md) 一致；不一致的地方要么改壳，要么同一个提交里改那份文档。
- 崩溃不丢：学习数据原子写、坏行跳过、边界 panic 隔离，与 [design/architecture.md](../design/architecture.md)「崩溃不丢」一致。
