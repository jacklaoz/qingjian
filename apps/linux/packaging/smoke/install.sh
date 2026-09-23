#!/bin/bash
# 在容器里跑（run.sh 调）：装 /pkgs 下的两个包，确认自启登记与插件依赖，起 Server 用 check.py 打 nihao，
# 等本地整句模型加载完，再卸载看自启登记有没有清掉。$1 = apt / dnf / zypper
set -e
case $1 in
  apt) export DEBIAN_FRONTEND=noninteractive; apt-get update -qq; apt-get install -y -qq systemd python3 >/dev/null 2>&1
       apt-get install -y -qq /pkgs/qingjian_*.deb /pkgs/qingjian-fcitx5_*.deb >/dev/null 2>&1 || { apt-get install -y /pkgs/*.deb 2>&1 | tail -6; exit 1; }
       echo "装上：$(dpkg-query -W -f='${Package} ${Version}  ' qingjian qingjian-fcitx5)" ;;
  dnf) dnf install -y -q systemd python3 >/dev/null 2>&1; dnf install -y -q /pkgs/*.rpm >/dev/null 2>&1 || { dnf install -y /pkgs/*.rpm 2>&1 | tail -6; exit 1; }
       echo "装上：$(rpm -q qingjian qingjian-fcitx5 | tr '\n' ' ')" ;;
  zypper) zypper -q -n install systemd python3 >/dev/null 2>&1; zypper -q -n --no-gpg-checks install --allow-unsigned-rpm /pkgs/*.rpm >/dev/null 2>&1 || { zypper -n --no-gpg-checks install --allow-unsigned-rpm /pkgs/*.rpm 2>&1 | tail -6; exit 1; }
       echo "装上：$(rpm -q qingjian qingjian-fcitx5 | tr '\n' ' ')" ;;
esac
echo "自启登记：$(readlink /etc/systemd/user/default.target.wants/qingjian-server.service || echo 没有)"
so=$(ls /usr/lib*/fcitx5/qingjian.so /usr/lib/*/fcitx5/qingjian.so 2>/dev/null | head -1); echo "插件：$so，缺库：$(ldd $so | grep -c 'not found')"
export XDG_RUNTIME_DIR=/tmp/run HOME=/root; mkdir -p $XDG_RUNTIME_DIR
qingjian-linux-server >/tmp/server.log 2>&1 &
for i in $(seq 1 60); do [ -S $XDG_RUNTIME_DIR/qingjian.sock ] && break; sleep 0.5; done
python3 /check.py nihao
for i in $(seq 1 90); do grep -rqs "整句模型.*加载\|模型已加载\|model.*loaded" /tmp/server.log /root/.local/state/qingjian/logs && break; sleep 0.5; done
echo "模型：$(grep -rhs "整句模型\|model" /tmp/server.log /root/.local/state/qingjian/logs | sed 's/\x1b\[[0-9;]*m//g' | grep -o '本地整句模型[^ ]*\|path=[^ ]*model[^ ]*' | head -2 | tr '\n' ' ')"
case $1 in apt) apt-get remove -y -qq qingjian-fcitx5 qingjian >/dev/null 2>&1 ;; *) rpm -e qingjian-fcitx5 qingjian ;; esac
echo "卸载后自启登记：$(readlink /etc/systemd/user/default.target.wants/qingjian-server.service || echo 已清)"
