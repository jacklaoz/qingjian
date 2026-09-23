#!/usr/bin/env bash
# 把 build.sh 打好的包拿到几个发行版的干净容器里装一遍并自检（install.sh），不碰本机：
#   apps/linux/packaging/smoke/run.sh
# 每个发行版打印：装上的版本、systemd 自启登记、插件缺不缺库、打 nihao 的候选、本地整句模型加载、卸载后自启登记是否清掉。
# 容器里没有图形会话，真会话里切到青简打字还是要人在机器上试。
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
out=$(cd -- "$here/../../../.." && pwd)/target/linux-pkg/out
failed=0
for spec in 'debian:trixie apt deb' 'ubuntu:26.04 apt deb' 'fedora:42 dnf rpm' 'fedora:latest dnf rpm' 'opensuse/tumbleweed zypper rpm'; do
  read -r image manager format <<<"$spec"
  pkgs=$(mktemp -d)
  cp "$out"/*."$format" "$pkgs/"
  echo "######## $image"
  docker run --rm -v "$pkgs:/pkgs:ro" -v "$here/install.sh:/install.sh:ro" -v "$here/check.py:/check.py:ro" \
    "$image" bash /install.sh "$manager" || failed=1
  rm -rf "$pkgs"
done
exit "$failed"
