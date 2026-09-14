# Linux 从零到能日用（2026-09-14）

## 起因

Phase 5 只剩 Linux 没开工。这一天把它从「一行代码没有」推到「装在开发机上当会话输入法用」：
`apps/linux` 建起来、走 IBus 接上 Core、打成三个 `.deb` 装机、切成系统输入法日常打字。
方案取舍与分期在 [plan/linux_plan.md](../plan/linux_plan.md)，各模块的实现要点在
[crate-notes.md](crate-notes.md)，这一页只记**过程里踩的坑和验出来的数字**。

## 怎么分层

```text
ibus/        D-Bus 那层：GVariant 序列化、工厂、引擎对象、总线地址
  ↑
dispatch/    按键 → 帧。不认 IBus 也不认 Wayland，所以能脱离输入法框架整段测
  ↑
assembly/    装 Engine
  ↑
Core
```

`dispatch` 这一刀切得值：80 个单元测试里绝大多数在这一层，不用起 daemon、不用 D-Bus。
按键规则是照 macOS 的 `handle_text` / `handle_command` 与 Windows 的 `apply_key` 手工对齐的——
**这是第三份孪生逻辑**，`assembly` 也是第三份。收口方案见 linux_plan.md 的 L3。

比 Windows 少了会话分派：IBus 一个引擎实例服务当前焦点，没有「一个 Server 服务多个应用进程」那回事，
所以 Router 只有一份组句状态，形状更接近 macOS 的进程级单例。

## 五个只有真机才暴露的坑

### 1. ibus-daemon 不给引擎进程设 `IBUS_ADDRESS`

本来以为 daemon 拉起引擎时会把总线地址放进环境变量，写完发现引擎起来就退出：找不到地址。
要自己读 `~/.config/ibus/bus/` 下的地址文件。

**而且不能扫目录随便挑**：那个目录里常年躺着别的输入法或上次会话留下的陈旧文件——
开发机上就有 fcitx 留的 `<机器id>-unix-0`，按字典序还排在当前会话的 `<机器id>-unix-wayland-0` 前面，
挑中它连上去一个信号都收不到。改成按 ibus 自己的规矩算文件名（`<机器 id>-<主机>-<显示标识>`），
算出来的不在才退回扫目录，且只认 `IBUS_DAEMON_PID` 还活着的。

### 2. Shift + 数字的 keysym 是 `!` 不是 `1`

缺省的删候选快捷键正是 Shift + 数字。照 macOS 那样按字符认数字，这条快捷键在 Linux 上**整个失效**，
而且失效得很安静——键被当成普通标点吃掉了。得同时用硬件键码按**键位**认（evdev 的 `KEY_1` + X11 的 8 偏移 = 10）。
Windows 用虚拟键码干的是同一件事，只是那边字符与键位天然分开，不容易想到 Linux 也要。

### 3. IBus 的嵌套 GVariant 要包变体

`IBusText` 的签名是 `(sa{sv}sv)`，第四项那个 `v` 得把属性表**包成变体**再塞；
直接塞结构体会内联成 `(sa{sv}av)`。签名错了**不报错**，只是候选窗静默不出现。
`av` 的元素同理。现在三种对象的签名由单元测试钉死，改错了先红的是测试。

### 4. `bundled_root` 把 `/` 当成了仓库根

装完第一次跑 `--check` 就炸出来：随包数据根解析成了 `/`，回落到 148 条的样例词库。
原因是开发布局的回退写成「exe 往上数三层」（`exe → {debug,release} → target → 仓库根`），
装到 `/usr/bin` 之后往上三层正好是 `/`，而这台机器根目录下有个 `/data`，`has_resources` 就认了。

症状是**装好了、能启动、就是一个词都打不出**——最难查的那种。改成先确认 exe 真在
`target/{debug,release}/` 里才认开发布局。这个函数 **Windows 也在用**，所以算顺手修了 Windows。

### 5. `Mutex` 毒化 = 崩一次就再也打不了字

D-Bus 那层要求接口对象 `Send + Sync`，而 `Engine` 因为几个 `RefCell` 缓存不是 `Sync`。
一开始按「另开一条工人线程持有它」设计，写之前测了一下发现 **`Router` 是 `Send` 的**，
套个 `Mutex` 就够，工人线程省了（`dispatch::tests` 里有一条编译期断言钉着）。

但满地 `lock().expect(...)` 埋了个雷：`Mutex` 一旦被 panic 毒化，之后每次 `lock()` 都返回错误。
按「崩溃不丢」那条，崩一次之后输入法该照常服务，而不是永久瘫掉。现在统一走 `Shared::lock`，
毒化了就把状态取回来接着用；按键外面再套一层 `catch_unwind`，拦下后清组句、这个键让给应用。

## 合成客户端验不了「用户看到什么」

写了个 D-Bus 客户端当「应用」跑端到端，候选表和上屏都收得到，**唯独带内容的 `UpdatePreeditText` 收不到**。
一路怀疑到序列化、面板、能力位，试了半天：起面板一样收不到，同一个 `IBusText` 在另外两条路上完全正常。

真相是**真实应用的 preedit 由 GTK / Qt 的输入法模块自己渲染**，不走「客户端订阅 InputContext 信号」这条路。
拿 D-Bus 客户端当应用来验 preedit，本来就验不到。换成开一个真的 gnome-text-editor，一分钟就看明白了。

**经验**：合成客户端能验协议对不对，验不了用户看到什么。后者只能开一个真应用，没有捷径。

## 验收

装机后（GNOME / Wayland，真产品数据：22 万词库 + 42 MB 语言模型 + 24 万词释义表）：

| 项 | 数字 |
|---|---|
| 每键耗时 | 最慢 **3.34 ms**、平均 **857 µs**（标准 10 ms 以内） |
| 整句转换 | `zhonghuarenmingongheguowansui` → 中华人民共和国万岁 |
| 引擎常驻 | 约 104 MB |
| 测试 | 84 个单元测试 + 1 个对着真 ibus-daemon 的端到端（没有 ibus 就跳过，CI 不受影响） |

日常打字验过的：词级、整句、简拼、拼写纠错（`zhonoghua` / `zonghua` 都救回了 中华）、
中英混输（`clone` 原样直通）、学习六张表落盘。

**X11 也能用**：方案 C 压根不碰显示服务器，候选窗的绘制与定位全是 IBus 面板的活。
`GDK_BACKEND=x11` 起的 XWayland 客户端里验过。计划里「只做 Wayland」说的是将来那一步自绘，不是整个 Linux 支持。

## 打包与安装

三个 `.deb`（`apps/linux/packaging/build-deb.sh`）：程序 2.1 MB、数据 27 MB、模型 49 MB。
分开是因为发行版不收一百多兆的单包，而且数据与模型的更新节奏跟代码不一样。
程序包不依赖后两个：缺语言模型退化成一元词频、缺模型不重排，Core 本来就支持。

装到 `/usr/bin/qingjian-linux` + `/usr/share/qingjian/{data,assets}`，组件 XML 落 `/usr/share/ibus/component/`，
postinst / postrm 调 `ibus write-cache --system` 让 ibus 重扫。

## 还没做的

- **`.rpm`**：没有能验的环境，不写没跑过的打包脚本。
- **L3 / L4**：自绘要先把 `renderer-spike` 分支合进 main（那份代码不在这台机器上），
  而 L4 的 `input-method-v2` 只有 wlroots 系实现，GNOME / KDE 验不了。
- **设置界面**：现在改设置是编辑 `~/.config/qingjian/config.toml`，热加载即时生效。
- **`[apps]` 按应用配置**：Wayland 拿不到前台应用标识，X11 可行但只覆盖一半平台，不做。
