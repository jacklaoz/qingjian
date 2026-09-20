# Core 与显示层拆解

[architecture.md](architecture.md) 写的是「为什么这么定」，这一页写「现在长什么样」：
`Engine` 的结构、一次按键从键到候选走过的每一层、三条异步旁路、上屏时的写回，以及显示层两端各画了什么。
读代码前先看这页，读完直接进 `crates/qingjian-core/src/engine/`。

2026-09-14 对着 main 写的。实现改了要回来改这里。

## 一、`Engine` 的五组字段

`Engine`（`engine/mod.rs`）是 Core 对外的唯一门面，约 60 个字段，分五组看：

| 组 | 字段 | 性质 |
|---|---|---|
| **数据** | `dictionary`、`extra_dictionaries`、`english`、`emoji`、`language_model` | 只读，启动装配一次 |
| **状态** | `composition`、`punctuation`、`english_mode`、`commit_chain`、`history`、`recent_commits` | 每键变 |
| **注入口** | `translator`、`english_translator`、`learner`、`predictor`、`sentence_scorer`、`rescorer`、`gloss_filler`、`input_logger`、`usage_meter`、`vocabulary_tracker` | 全是 `Box<dyn Trait>`，全有 `No*` 空实现 |
| **配置** | `fuzzy`、`modes`、`shuangpin`、`zhuyin`、`interpolation`、`typo_costs`、`neural_weight` / `neural_margin` / `neural_context` | 壳热加载时 `set_*` 推进来 |
| **缓存** | `span_cache`、`last_query`、`correction_cache`、`neural_cache`、`last_rescored` | 全是 `RefCell` / `Cell` |

两条对移植与并发都要紧的性质：

- **注入的 trait 全部 `Send`**（`Translator` / `Learner` / `Predictor` / `SentenceScorer` / `InputLogger` /
  `UsageMeter` / `VocabularyTracker` / `GlossFiller`），缺省实现全是空操作。所以 CLI 和单元测试不需要真实词典、
  不联网、不落盘也能跑完整条查询路径——这是整个架构最值钱的一处设计。
- **`Engine` 自己因为那 5 个内部缓存是线程亲和的，不是 `Send`。** macOS 用主线程 `thread_local` 单例
  （`host::with`），Windows 用 Server 的一条工人线程独占。任何新平台壳都必须把 Engine 钉在一条线程上。

## 二、一次按键的数据流

```text
壳: push(c) / backspace / move_cursor …
      │
      ▼
composition.rs      拼音缓冲 + 光标 + scope（光标停中间时只算光标前那段）
      │
      ▼
Engine::query()     engine/query/mod.rs，最热的一条路
      │
      ├─ shuangpin::decode / zhuyin::decode ──► Decoded{pinyin, tail, keys_for()}
      │     双拼两键一音节、大千注音键位表都先解成全拼，之后全部复用全拼路径
      │
      ├─ parser::segment      trie 切音节；全拼 / 简拼混排 / 末尾残缺音节
      │     fuzzy::Expanded 把每个位置扩成多种写法（z/zh、an/ang…），词库一次查出
      │     整段切不动就取最长可切前缀，剩下的字母留作未切分尾部
      │
      ├─ correction           一处编辑变体（换位 / 换字母 / 多一个 / 少一个）
      │     先过无分配的 parser::is_fully_segmentable 筛掉上千变体
      │     再按噪声信道挑：纠正后整句得分扣掉编辑代价仍高于原样才纠
      │
      ├─【词级】dictionary.lookup_pattern / lookup_exact
      │     主词库 + 领域词库 + 用户导入词库 + 用户词，逐级前缀二分收窄
      │     ranking：log P(词 | 上一个上屏词) + 封顶的选择次数加分 + 非末尾简拼少的优先
      │
      ├─【整句】sentence/
      │     词图：每格 lookup_exact，每格留词频前 6（含简拼位置的格子留 20）
      │          ├─ SpanCache 跨按键复用
      │          └─ correction::typo 敲错边：每个完整音节的一处敲错变体带代价入图
      │     Viterbi + 束搜索（束宽 8）
      │          打分 = (1−μ)·静态 bigram + μ·个人 n-gram，μ = c(v)/(c(v)+8) 封顶 0.5
      │          个人侧先算二元（0.8 / 0.2 插值），上文 (u,v) 见过再套绝对折扣三元（D = 0.75）
      │     ──► Conversion{paths, penalty}
      │
      ├─ english / emoji / shortcut / custom_phrase 各自插候选
      │
      ▼
CandidateList{items: Vec<Candidate>}
      │
      ▼
Engine::annotate()  Translator 查本地表 ──► Translation{senses}
      │             VocabularyTracker 标 Sense::fresh（看到轮次不到 FRESH_UNTIL 即生词）
      ▼
壳: 画一帧
```

各层的取舍与常数为什么是这些值，见 [architecture.md](architecture.md)「Core 的关键技术决定」与
[notes/constant-sweep.md](../notes/constant-sweep.md)。

## 三、三条异步旁路

三条都遵守同一条铁律：**不阻塞候选生成，也不重排已有候选**。

| 旁路 | trait | 线程 | 壳侧轮询 | 丢不丢得起 |
|---|---|---|---|---|
| 云联想 | `Predictor` | `qingjian-predict` 自开，防抖 300 ms、缓存 64、超时 5 s | `poll_prediction()` | **丢得起**：最新请求优先，Engine 给序号只认最新 |
| 神经重排 | `SentenceScorer` | `rescoring::RescoreWorker` + `NeuralCache` | `request_rescoring()` → `poll_rescoring()` | 丢得起：壳停键 80 ms 才送任务 |
| 释义兜底 | `GlossFiller` | `qingjian-predict` 独立线程，攒 1.5 s 或 8 个词 | `poll_glosses()` | **丢不起**：每个词都要问到，慢点没关系 |

同样是「异步调云」，因为丢得起和丢不起，`Predictor` 与 `GlossFiller` 是两个 trait、两条线程、两套策略，
没有合并成一个通用的异步任务队列。这条分界是有意的，见 [architecture.md](architecture.md)「翻译数据本地化」。

## 四、上屏时的五路写回

`Engine::commit()`（`engine/commit/`）一次触发：

```text
commit(candidate)
 ├─ Learner::record          词频 / 选择次数 / 按输入串的选择（user-choices.tsv）
 ├─ Learner::user_ngram      词转移：用户点选记双份，整句路径顺带的记一份
 ├─ 自动造词                  连选两词、合起来 ≤ 4 字且词库与用户词都没有 ──► user-words.tsv
 ├─ Learner::record_typo     接受的 (敲的, 要的) 音节对 ──► user-typos.tsv
 ├─ InputLogger::log         input-log.jsonl 一行（键 / 切分 / 前 5 候选 / 选了第几个 / 来源 / ms / app…）
 ├─ UsageMeter::record       汉字 / 中文词 / 英文词计数 ──► usage.tsv
 └─ VocabularyTracker        译词的「上屏过」计数 ──► user-vocab.tsv
```

两层包装罩在外面：

- **私密输入**：`Engine::set_private(true)` 让 `MutedLearner` / `MutedInputLogger` 生效——**写吞掉、读照常、排序不变**，
  联想 / 翻译 / 释义兜底也不发。Windows 靠 TSF 的输入范围判定，macOS 靠 Secure Input。
- **退格撤销**：`recent_commits` 留最近 4 次上屏，壳把组句外的退格用 `note_backspace()` 告诉 Engine；
  删光再重打同一段拼音换了个词，上面五路写回**全部退回**。

## 五、六处缓存

性能全部来源于此，动其中任何一处都要用 `qingjian-cli --typing` 的 release 构建重量每键耗时：

| 缓存 | 键 | 失效时机 |
|---|---|---|
| `span_cache`（`sentence::SpanCache`） | 格子模式（含模糊写法） | commit / `learner_mut` / 换 Learner |
| `last_query`（`QuerySnapshot`） | 当前 scope | 每次 `push` |
| `correction_cache` | scope 字符串 | 每次 `push` |
| `neural_cache` | 前文 + 整句文本 | 前文变 |
| 词库前缀记忆化 | 前缀模式 | 单次查询内 |
| `lm` 的 CSR bigram | — | 只读，随 `.qj` mmap |

历次优化的起因、定位方法与前后数字在 [notes/performance.md](../notes/performance.md)。

## 六、显示层

### Core 交出什么

壳拿到的是**已经排好序、已经带好 annotation** 的数据，**一行排序逻辑都不在壳里**：

```text
CandidateList{ items: Vec<Candidate> }
  Candidate{ text, kind: CandidateKind, translation: Option<Translation>, reading: Option<String>, … }
    Translation{ senses: Vec<Sense> }
      Sense{ part_of_speech, gloss, reading, fresh }
        .furigana() -> Vec<Segment{text, reading}>      日文汉字段注平假名

CandidateLayout / Cell        分页排布：本地候选 + 云端固定槽位（[predict] slots）
MarkedSegment{text, kind}     preedit 分段：普通 / 纠错删除线 / 英文直输段 …
```

连「云端词补进第一页末尾几格、前面的本地候选一格不挪」这种排布规则都在 Core 的 `CandidateLayout` 里。

### 候选窗只有四个原语

若干行 (序号, 候选词, 译文) / 一行高亮 / 跟随光标定位 / 异步补画译文。
技术选型与被否掉的方案（WebView、egui / iced / GPUI）见 [candidate-ui.md](candidate-ui.md)。

### 两端各画了什么

| | macOS | Windows |
|---|---|---|
| 进程 | 输入法进程内，主线程 | **Server 进程**一条专用 UI 线程（HWND 只在该线程碰） |
| 窗口 | 非激活 `NSPanel`，level 101（与系统候选框同级） | GDI 分层窗 + `UpdateLayeredWindow` |
| 绘制 | `NSAttributedString` / Core Text | GDI，自己合成预乘 alpha 的 BGRA 位图 |
| 圆角阴影 | AppKit | `ui/layered/` 手写：定向主阴影 + 环境光晕 |
| 置顶 | `CanJoinAllSpaces` + `FullScreenAuxiliary` | exe 的 `uiAccess` manifest + 代码签名 + 装 Program Files |
| 定位 | IMK `attributesForCharacterIndex:lineHeightRectangle:` | DLL 量 `GetTextExt` ──► `PositionCandidates{rect}` |
| 踩过的坑 | macOS 26 把面板绑死在创建时已有的 Space，全屏应用里看不见；每次 `orderFront` 后查 `isOnActiveSpace`，不在就把内容视图搬到新建面板 | 普通置顶窗被微软商店 / 任务栏搜索的高 z-band 盖住；整个渲染层从 DLL 搬进 Server 进程 |

### 已知的三份重复

`apps/macos/src/candidates/row.rs` 与 `apps/windows/server/src/ui/candidates/row.rs` 的 `Tone`、`Row`、
`from_candidate()` 一字不差（后者文件头就写着「与 macOS 端一致」），`Frame` 与 `Theme` 同理；
`apps/windows/server/src/dispatch/key/input.rs` 的文件头写着「分流规则与 macOS 壳的 `handle_text` / `handle_command` 对齐」。

这两处重复各自的收口方案：显示面是 `crates/qingjian-render`（一帧 + 主题 → 位图，各平台只贴图，
见 [notes/crate-notes.md](../notes/crate-notes.md)）；按键分流面还没有收口方案，
Linux Server 是第三份，见 [notes/linux-fcitx5.md](../notes/linux-fcitx5.md)。
