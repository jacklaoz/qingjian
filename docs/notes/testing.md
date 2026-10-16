# 验证方法与环境

改完代码到开 PR 之间跑什么、在哪跑、怎么跑。脚本都在 `tools/verify/`，总入口 `tools/verify/run.sh`
按与 main 相比改了的文件挑检查项；结果（日志、截图、汇总表）都落在 `target/verify/`。

提交钩子（`.githooks/`）与 CI 是这套东西的子集：钩子管每次提交都要过的，CI 管 PR 里的三个平台；
图形端到端、FreeBSD 全量与 Mac 真机只能在本机或测试机上跑，由 `run.sh` 负责。

## 检查项

| 项 | 脚本 | 覆盖什么 | 什么时候自动挑上 | 耗时（增量 / 首次） |
|---|---|---|---|---|
| `local` | `local.sh` | fmt、clippy、全 workspace 测试（范围同钩子）、安装脚本测试、设置程序截图脚本 | 每次 | 1 分钟 / 5 分钟 |
| `cross` | `cross.sh` | 在本机对 macOS（整个 workspace）、FreeBSD、Windows（gnu）做 clippy 类型检查 | 改了 `crates/`、`apps/`、依赖 | 30 秒 / 2 分钟 |
| `fcitx5-docker` | `fcitx5-docker.sh` | Ubuntu 26.04 容器里编 Fcitx5 插件、跑它的 ctest（48 个，含对真 Server 的测试） | 改了 `apps/linux/`、Core、platform、锁文件 | 1 分钟 / 2 分钟 |
| `freebsd` | `freebsd/check.sh` | FreeBSD 虚拟机里：按 `data.lock` 取数据、`install.sh` 安装、全 workspace 测试、插件 ctest、安装脚本测试 | 改了 Linux / FreeBSD 共用的代码 | 5 分钟 / 15 分钟 |
| `freebsd-e2e` | `freebsd/e2e.sh` | 图形端到端：Plasma X11、XFCE X11、嵌套 sway（Wayland）× Mousepad / FeatherPad / Kate / Firefox，跑 `cases.tsv` | 改了输入链路（Linux 前端、Core、platform、用例） | 15 分钟 |
| `mac` | `mac/remote.sh` | Mac 测试机上整个 workspace 的 clippy 与测试（同 Mac 上的钩子） | 改了 macOS 壳、共享 crate、依赖 | 2 分钟 / 12 分钟 |
| `mac-e2e` | `mac/remote.sh --e2e`，装好后 `--cases` | 打包装好（已有 `/Library` 下的 pkg 版时打 pkg、停下等人用 sudo 装），切到青简，osascript 往 TextEdit 发 `cases.tsv` 的按键、读回比对；先核对装着的就是当前提交 | 只手动（`--only mac-e2e`） | 15 分钟 |

`run.sh --list` 只列这次会跑哪些、为什么；`--all` 全跑（不含 `mac-e2e`）；`--only freebsd,mac` 只跑指定的。
环境没准备好的项会跳过并说明怎么准备，不算失败。

## 环境准备

- **交叉类型检查**：`rustup target add aarch64-apple-darwin x86_64-unknown-freebsd x86_64-pc-windows-gnu`（`cross.sh` 缺了会自己加）。
  装了前两个，pre-commit 会在相关提交上自动跑 darwin（整个 workspace）与 FreeBSD 的检查。
- **Docker**：本机能跑 `docker`（用户在 docker 组）。镜像第一次自动构建，缓存为 `qingjian-fcitx5-test:26.04`；
  `fcitx5-docker.sh ubuntu:24.04` 换成 CI runner 的 Fcitx5 版本。容器用本机编的 Server，所以容器的 glibc 不能比本机旧。
- **FreeBSD 虚拟机**：要 KVM（用户在 kvm 组）与 `qemu-system-x86 qemu-utils xorriso socat`。
  ```sh
  tools/verify/freebsd/vm.sh create      # 下官方 cloud-init 镜像、校验、建机、等首次开机（打补丁）
  tools/verify/freebsd/provision.sh      # 装 packages.txt（约 3 GB）、桌面、Rust；可重复跑
  tools/verify/freebsd/vm.sh start --gui # 想亲眼看桌面时：本机开一个 QEMU 窗口；不带 --gui 无界面跑，截屏照样能用
  ```
  虚拟机目录缺省 `~/vms/freebsd`（`QINGJIAN_FREEBSD_VM`），SSH 转发到本机 2222，桌面用户与本机同名、密码 `qingjian`。
  `vm.sh ssh` 登录、`vm.sh shot 文件.png` 截屏、`vm.sh destroy` 删机重来。
  QEMU 用户态网络的端口转发在短时间大量新建连接时偶尔卡在握手（`Connection timed out during banner exchange`），
  所以脚本里的 ssh 共用一条连接（`ControlMaster`），连接级失败重试三次；看到「ssh 连不上虚拟机」提示连着出现，再查虚拟机本身。
  测试里会在后台常驻的程序（`wl-copy`、`xclip`）一定要把输出接走，否则 ssh 等它们退出，宿主那头就卡住。
  测试应用每次启动前全部关掉（Kate 收到 SIGTERM 不退，两秒后补 SIGKILL），Mousepad 的会话恢复关掉（被杀后会弹「是否恢复数据」抢走焦点）；
  Plasma 的会话总线不在环境变量里，`insession` 取桌面里 fcitx5 连着的那条，不能取 `/tmp` 下最新的 dbus 套接字（嵌套 sway 也会建）。
- **Mac 测试机**：`QINGJIAN_MAC_HOST=user@host`，免密 SSH；同步到那边的 `~/work/qingjian-verify`（`QINGJIAN_MAC_DIR`），不碰手工用的目录。
  没同意 Xcode 许可也能跑（脚本走 Command Line Tools）。git worktree 的 `.git` 拷过去是坏的，脚本改传 git bundle 在那边建仓库：
  bundle.sh 的构建号是提交数，退成 1 的话 pkg 装不上（安装器不拿构建号小的覆盖已装版本，`/var/log/install.log` 里是 `Skipping component`）。
  那台 Mac 的磁盘紧，debug 与 release 两份产物放不下，所以缺省模式只跑检查、`--e2e` 只打包。
  那台 Mac 是日常用的：`--cases` 先把 `~/Library/Application Support/Qingjian` 整个挪开、输入法以全新状态（缺省配置、没有学习数据）跑，
  跑完强制结束输入法（不让它把测试期间学到的写回去）再原样挪回，中途出错也会挪回；前后对过校验值，一个字节不差。
  输入法进程被结束后菜单里会暂时没有它，脚本用 `qingjian-macos --register` 重新启用，再等日志里出现「本地模型已加载并预热」才开始。
  排查时 `QINGJIAN_CASES=文件` 换用例、`QINGJIAN_MAC_DEBUG=1` 让输入法打调试日志并取回 `target/verify/mac-ime.log`。`--e2e` 前提：当前输入法已是青简、sshd 有辅助功能权限；
  它会占用那台 Mac 的键盘，并替换已装的青简，新装的要注销重登一次才出现在输入法菜单里。

## 图形端到端怎么判对错

用例在 `tools/verify/cases.tsv`，FreeBSD 与 Mac 共用：第一列是按键（`SPACE` `RET` `ESC` `BS` `WAIT` 是键，其余逐字敲），第二列是期望读回的文本。
每条用例：清空输入框并确认真的空了 → 剪贴板放哨兵 → 切到青简 → 发键 → 全选复制 → 读回比对（读回的还是哨兵就是空）。
不看截图判对错；截图（`target/verify/freebsd-e2e/*.png`）留给人看候选窗口的样子。
失败的用例自动重试一次：重试才过的记「重试才过」，在汇总里单列、不判整轮失败（嵌套 sway 经外层 X11 收键，机器忙时会丢键），
同一条连着几轮都要重试才过就当真问题查；两次都错才是失败。

期望值依赖正式词库的排序。换了词库或改了排序后对不上，先确认是不是排序回退，确实是改进再改用例。
新功能、修过的按键行为，往 `cases.tsv` 里加一条能复现的用例。

## 修 bug 的验证流程

1. 先复现：能用单元测试 / 回放 / `cases.tsv` 写出来的先写出来，看着它失败；只有真机能复现的，在 issue 里写清系统、应用与步骤。
2. 修。
3. `tools/verify/run.sh`，全过再提交；改了输入链路的至少过 `freebsd-e2e`，改了 macOS 壳或共享 crate 的过 `mac`。
4. PR 的「怎么验证的」贴 `run.sh` 最后的结果表，写明哪些项跳过了、为什么；涉及真机的写系统版本、应用与步骤。

## 这些检查是怎么补出来的

| 来源 | 当时漏了什么 | 现在由谁兜 |
|---|---|---|
| #628 第 1 条 | CI 的 macOS job 与 Linux 上的 darwin 检查都只查 IMK 壳，Linux Server 在 Mac 上编不过，要到 Mac 上提交才发现 | CI macOS job 与 pre-commit 的 darwin 检查都改成整个 workspace；`cross darwin`；`mac` |
| #625 / #661 | 新版本读不了开发版写下的配置（`traditional = "off"`），整份配置回缺省；没有「旧配置必须能读」的测试 | `qingjian-platform` 的历代配置回归测试（`tests/config_history.rs`） |
| FreeBSD 移植 | 没有任何 BSD 检查：mio / libc 不兼容、`SO_PEERCRED`、`/proc`、libc++ 的 `std::function` 都是手工发现的 | CI 与 pre-commit 的 FreeBSD 类型检查；`freebsd`；`freebsd-e2e` |
| 合并 main 时 | 设置程序截图脚本在 Linux 上去点不存在的「云服务」统计页，中途失败 | 修了脚本；CI 与 `local` 实际跑一遍 |
| 合并 main 时 | 本地 `data/generated` 与 `data.lock` 不是同一版，装到别处才报校验失败 | `freebsd` 一律按 `data.lock` 下载校验 |
| Qt / Firefox / KDE / Wayland 手工验证 | 全靠手打和截图，换个提交就得重来 | `freebsd-e2e` |

## 还覆盖不到的

- **KDE Plasma 的 Wayland 会话**：虚拟机里没有 `/dev/dri`，整机 Wayland 起不来；嵌套的 KWin 不把按键交给输入法（见 [linux-fcitx5.md](linux-fcitx5.md) 的 FreeBSD 一节）。要真机。
- **IBus 前端**：在 #628 的分支上，合进来之后给 `freebsd/e2e.sh` 加一个 IBus 会话；#628 的「组字到一半换输入框」要在那时写成用例。
- **Windows**：没有测试机，只有 CI 与 `cross windows` 的编译检查；按键、上屏要真机。
- **Linux 真桌面**：本机的 GNOME Wayland 是日常用的会话，不在上面自动发键；Linux 的图形链路靠 FreeBSD 虚拟机里同一套前端代码兜着，发行版差异要真机。
