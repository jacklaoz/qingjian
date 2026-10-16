#!/usr/bin/env bash
# //! 在 Mac 测试机上跑：同步当前工作区 → 整个 workspace 的 clippy 与测试（与 Mac 上的 pre-commit / pre-push 同一范围）。
# //! --e2e 不跑检查（那是缺省模式的事，Mac 磁盘也放不下 debug 与 release 两份产物），同步后打包装上，用 osascript 往 TextEdit 发 cases.tsv 的按键、读回文本比对。
# 用法：QINGJIAN_MAC_HOST=user@host remote.sh [--e2e | --cases]；远端目录缺省 ~/work/qingjian-verify（QINGJIAN_MAC_DIR），不和手工同步的 ~/work/qingjian 混用
#   --e2e    打包安装再跑用例。/Library 下已有 pkg 版时只能装 pkg（要 sudo），打好包、打印安装命令后停下（退出码 3）；
#            ~/Library 下的开发版（或都没装）就直接 bundle.sh --install。新装的输入法要注销重登一次才出现在菜单里。
#   --cases  不同步不编译，只对已装的版本跑用例；先核对已装版本的短哈希与当前提交一致，免得测的是别的版本。
# 端到端会占用那台 Mac 的键盘（两边按键会穿插），跑之前先确认没人在用它。
set -euo pipefail
# shellcheck source=../lib.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/../lib.sh"
host=${QINGJIAN_MAC_HOST:?要设 QINGJIAN_MAC_HOST=user@host}
mode=${1:-check}
case "$mode" in check|--e2e|--cases) ;; *) sed -n '2,9p' "$0" | sed 's/^# //'; exit 2 ;; esac
remote_dir=${QINGJIAN_MAC_DIR:-'~/work/qingjian-verify'}
# Xcode 许可没同意时 /usr/bin/git 与 cc 拒绝运行，改走 Command Line Tools；cargo 在 ~/.cargo/bin
remote_env='export DEVELOPER_DIR=/Library/Developer/CommandLineTools PATH="$HOME/.cargo/bin:$PATH"'

mac() { ssh -o BatchMode=yes -o ConnectTimeout=8 "$host" "$remote_env; $*"; }

step "连接 $host"
mac 'sw_vers -productVersion; uname -m' | paste -sd' ' || die "连不上 $host"

step '同步工作区（带 .git：bundle.sh 要取短哈希；不碰那边的 target/ 与 data/，产品数据由 data-fetch.sh 取）'
# .git 不直接拷：git worktree 的 .git 只是指向主仓库的文件，拷过去是坏的。改成把当前分支打成 git bundle，
# 在那边建一个真仓库并指到同一个提交（只重置索引、不动同步过去的文件）。bundle.sh 的构建号是提交数，
# 退成 1 的话 pkg 装不上去：安装器不拿构建号小的覆盖已装的版本（install.log 里是 Skipping component）
rsync -a --delete --exclude /target --exclude /data --exclude /.git --exclude .flatpak-builder --exclude .codegraph \
  -e 'ssh -o BatchMode=yes' "$repo_root/" "$host:${remote_dir#\~/}/"
bundle=$(mktemp --suffix=.bundle)
git -C "$repo_root" bundle create -q "$bundle" HEAD
scp -q -o BatchMode=yes "$bundle" "$host:/tmp/qingjian-verify.bundle"
rm -f "$bundle"
mac "cd $remote_dir && { [ -d .git ] || git init -q; } && git fetch -q /tmp/qingjian-verify.bundle HEAD && git reset -q FETCH_HEAD && rm -f /tmp/qingjian-verify.bundle && echo \"那边的仓库：\$(git rev-parse --short HEAD)，\$(git rev-list --count HEAD) 个提交\""

if [[ "$mode" == check ]]; then
  step 'clippy（整个 workspace，与 Mac 上的 pre-commit 一致）'
  if mac "cd $remote_dir && cargo clippy --workspace --all-targets --locked -- -D warnings" > "$verify_out/mac-clippy.log" 2>&1; then
    ok 'clippy'
  else
    grep -E '^(error|warning)' "$verify_out/mac-clippy.log" | sort | uniq -c | head -20 >&2
    die "Mac 上 clippy 失败，完整输出：$verify_out/mac-clippy.log"
  fi

  step '测试（整个 workspace，与 Mac 上的 pre-push 一致）'
  if mac "cd $remote_dir && cargo test --workspace --locked" > "$verify_out/mac-test.log" 2>&1; then
    ok "$(awk '/^test result/ {p += $4; f += $6} END {print p " 个测试通过，" f " 个失败"}' "$verify_out/mac-test.log")"
  else
    grep -E 'FAILED|panicked|^error' "$verify_out/mac-test.log" | head -20 >&2
    die "Mac 上测试失败，完整输出：$verify_out/mac-test.log"
  fi
  exit 0
fi


app="/Library/Input Methods/Qingjian.app"
if [[ "$mode" == --e2e ]]; then
  if mac "test -d '$app'"; then
    step '已有 pkg 版（/Library）：打 pkg，装它要 sudo'
    mac "cd $remote_dir && tools/release/data-fetch.sh && apps/macos/scripts/bundle.sh --pkg" > "$verify_out/mac-bundle.log" 2>&1 \
      || die "打包失败，完整输出：$verify_out/mac-bundle.log"
    pkg=$(grep -oE '/[^ ]+\.pkg$' "$verify_out/mac-bundle.log" | tail -1)
    echo "在能输密码的终端里装（或在那台 Mac 上直接打开这个 pkg）："
    echo "  ssh -t $host 'sudo installer -pkg $pkg -target /'"
    echo "装好后：QINGJIAN_MAC_HOST=$host $0 --cases"
    exit 3
  fi
  step '打包并装到 ~/Library/Input Methods'
  mac "cd $remote_dir && tools/release/data-fetch.sh && apps/macos/scripts/bundle.sh --install" > "$verify_out/mac-bundle.log" 2>&1 \
    || die "打包失败，完整输出：$verify_out/mac-bundle.log"
  app="$HOME/Library/Input Methods/Qingjian.app"
fi

step '核对装着的是这个提交'
expected=$(git -C "$repo_root" rev-parse --short HEAD)
installed=$(mac "/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' '/Library/Input Methods/Qingjian.app/Contents/Info.plist' 2>/dev/null \
  || /usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' ~/'Library/Input Methods/Qingjian.app/Contents/Info.plist'")
[[ "$installed" == *"$expected"* ]] || die "装着的是 $installed，不是当前提交 $expected（pkg 没装上？看那边的 /var/log/install.log）"
ok "$installed"

step '切到青简'
here=$(dirname -- "${BASH_SOURCE[0]}")
scp -q -o BatchMode=yes "$here/select-input.applescript" "$here/textedit.applescript" "$host:/tmp/"
mac "osascript /tmp/select-input.applescript 青简" || die '输入法菜单里没有「青简」：在系统设置 → 键盘 → 输入法里添加，或注销重登一次'

step 'TextEdit 端到端：发按键、读回文本（别碰那台 Mac 的键盘）'
status=0
while IFS=$'\t' read -r keys expected; do
  [[ -z "$keys" || "$keys" == \#* ]] && continue
  actual=$(mac "osascript /tmp/textedit.applescript '$keys'" < /dev/null) || actual='（osascript 出错）'
  if [[ "$actual" == "$expected" ]]; then ok "$keys → ${actual:-（空）}"; else fail "$keys：期望「$expected」，实际「$actual」"; status=1; fi
done < "$here/../cases.tsv"
exit "$status"
