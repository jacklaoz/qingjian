#!/usr/bin/env bash
# //! 仅删除本安装清单中的文件，保留配置、词库、学习数据。
set -euo pipefail
install_prefix="$HOME/.local"
while (($#)); do
  case "$1" in
    --prefix) install_prefix=${2:?--prefix 需要路径}; shift 2 ;;
    --help) echo '用法：uninstall.sh [--prefix 安装时的绝对目录]'; exit 0 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done
# 先停服务再删文件：单元文件被删之后 systemd 就不认这个名字了，停不掉的 Server 会一直占着 socket
if command -v systemctl >/dev/null 2>&1 && systemctl --user list-unit-files qingjian-server.service >/dev/null 2>&1; then
  systemctl --user disable --now qingjian-server.service 2>/dev/null || true
fi
python3 "$(dirname -- "${BASH_SOURCE[0]}")/files.py" uninstall "$install_prefix"
command -v systemctl >/dev/null 2>&1 && systemctl --user daemon-reload || true
echo '卸载完成；用户数据已保留。fcitx5 重启一下、IBus 要重新登录一次（组件路径是会话启动时读的）才会忘掉这份青简。'
