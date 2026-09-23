#!/usr/bin/env python3
# 冒充输入法前端对 Server 打一串键，打印最后一帧的前几个候选；有候选退出码 0。
# 不经过输入法框架，用来确认 Server 起着、词库找得到：python3 check.py [拼音]（缺省 nihao）
# 消息顺序：开会话 → 握手 → 能力 → 焦点 → 逐键 → 失焦 → 关会话；每条消息是 4 字节小端长度 + JSON。
import json, os, socket, struct, sys
typed = sys.argv[1] if len(sys.argv) > 1 else 'nihao'
path = os.environ.get('QINGJIAN_SOCKET') or os.path.join(os.environ.get('XDG_RUNTIME_DIR', '/tmp'), 'qingjian.sock')
s = socket.socket(socket.AF_UNIX); s.connect(path)
def recv_exact(n):
    b = b''
    while len(b) < n:
        d = s.recv(n - len(b))
        if not d: raise SystemExit('Server 断开')
        b += d
    return b
def call(msg, reply=True):
    data = json.dumps(msg).encode(); s.sendall(struct.pack('<I', len(data)) + data)
    if reply:
        return json.loads(recv_exact(struct.unpack('<I', recv_exact(4))[0]))
ev = lambda e: {'LinuxEvent': {'event': e, 'session': 1}}
call({'OpenSession': {'app': None, 'protocol': 6, 'session': 1}})
call({'LinuxHello': {'context': '/qingjian/smoke', 'generation': 1, 'session': 1, 'version': 3}})
call(ev({'Capabilities': {'disabled': False, 'password': False, 'sensitive': False}}))
call(ev({'Focus': {'focused': True}}))
mods = {m: False for m in ('alt', 'caps', 'ctrl', 'english_mode', 'shift', 'win')}
for c in typed:
    last = call(ev({'Key': {'event': {'character': c, 'modifiers': mods, 'virtual_key': ord(c)}, 'release': False}}))
call(ev({'Focus': {'focused': False}}))
call({'CloseSession': {'session': 1}}, reply=False)
items = last['KeyResult']['frame']['candidates']['items']
print(f'打了「{typed}」，候选：' + ' '.join(i['text'] for i in items[:6]) if items else '没有候选')
sys.exit(0 if items else 1)
