#!/usr/bin/env bash
# //! 在 Mac 测试机上跑：同步当前工作区 → 整个 workspace 的 clippy 与测试（与 Mac 上的 pre-commit / pre-push 同一范围）。
# //! --e2e 再打包装到 ~/Library/Input Methods，用 osascript 往 TextEdit 发按键、读回文本比对。
# 用法：QINGJIAN_MAC_HOST=user@host remote.sh [--e2e]；远端目录缺省 ~/work/qingjian-verify（QINGJIAN_MAC_DIR），不和手工同步的 ~/work/qingjian 混用
# --e2e 会占用那台 Mac 的键盘（两边按键会穿插），并替换已装的青简；新装的输入法要注销重登一次才出现在菜单里。
set -euo pipefail
# shellcheck source=../lib.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/../lib.sh"
host=${QINGJIAN_MAC_HOST:?要设 QINGJIAN_MAC_HOST=user@host}
e2e=false
[[ "${1:-}" == --e2e ]] && e2e=true
remote_dir=${QINGJIAN_MAC_DIR:-'~/work/qingjian-verify'}
# Xcode 许可没同意时 /usr/bin/git 与 cc 拒绝运行，改走 Command Line Tools；cargo 在 ~/.cargo/bin
remote_env='export DEVELOPER_DIR=/Library/Developer/CommandLineTools PATH="$HOME/.cargo/bin:$PATH"'

mac() { ssh -o BatchMode=yes -o ConnectTimeout=8 "$host" "$remote_env; $*"; }

step "连接 $host"
mac 'sw_vers -productVersion; uname -m' | paste -sd' ' || die "连不上 $host"

step '同步工作区（带 .git：bundle.sh 要取短哈希；不碰那边的 target/ 与 data/，产品数据由 data-fetch.sh 取）'
# git worktree 的 .git 只是指向主仓库的文件，拷过去是坏的；这时不带它，bundle.sh 的构建号退成 unknown
git_excludes=()
[[ -f "$repo_root/.git" ]] && { git_excludes=(--exclude /.git); echo '工作区是 git worktree，不同步 .git（关于页的构建号会是 unknown）'; }
rsync -a --delete --exclude /target --exclude /data --exclude .flatpak-builder --exclude .codegraph "${git_excludes[@]}" \
  -e 'ssh -o BatchMode=yes' "$repo_root/" "$host:${remote_dir#\~/}/"

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

$e2e || exit 0

step '取正式数据、打包并装到 ~/Library/Input Methods（新装的要注销重登一次才出现在输入法菜单）'
mac "cd $remote_dir && tools/release/data-fetch.sh && apps/macos/scripts/bundle.sh --install" > "$verify_out/mac-bundle.log" 2>&1 \
  || die "打包失败，完整输出：$verify_out/mac-bundle.log"

step 'TextEdit 端到端：发按键、读回文本'
script=$(dirname -- "${BASH_SOURCE[0]}")/textedit.applescript
scp -q -o BatchMode=yes "$script" "$host:/tmp/qingjian-textedit.applescript"
status=0
while IFS=$'\t' read -r keys expected; do
  [[ -z "$keys" || "$keys" == \#* ]] && continue
  actual=$(mac "osascript /tmp/qingjian-textedit.applescript '$keys'") || actual='（osascript 出错）'
  if [[ "$actual" == "$expected" ]]; then ok "$keys → $actual"; else fail "$keys：期望「$expected」，实际「$actual」"; status=1; fi
done < "$(dirname -- "${BASH_SOURCE[0]}")/../cases.tsv"
exit "$status"
