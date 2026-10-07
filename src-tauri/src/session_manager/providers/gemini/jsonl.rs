//! Gemini CLI 会话文件的读取与回放。
//!
//! gemini-cli 自 2026-04-09（PR #23749，commit f744913）起把会话写成 JSONL 追加日志：
//! 消息每次改动都整条重写一行，另有 `$set`（元数据浅合并）、`$rewindTo`（回退）和
//! `$patch`（2026-10-01 起，commit d1cc08a）三种控制行；2026-10-01 之前用
//! `{"$set":{"messages":[...]}}` 做全量 checkpoint。这里按官方
//! `chatRecordingService.ts` 里 `createJsonlRecordAccumulator` 的规则回放
//! （核对的源码：gemini-cli fb972b2f，2026-10-02）：
//!
//! - 坏行、写到一半的末行直接跳过；
//! - 消息按 id 放进有序映射：重复 id 原位整条替换（位置取第一次出现），被删掉后再出现排到末尾；
//! - `$rewindTo` 删掉该 id 及其后所有消息，id 不存在时清空全部消息；
//! - 合并后的元数据缺 `sessionId` 或 `projectHash` 时，把整个文件当旧格式 JSON 解析。
//!
//! 旧格式 `session-*.json`（2026-04-09 之前）是一整份 JSON
//! （`{sessionId, projectHash, startTime, lastUpdated, messages[], ...}`），整份解析。
//!
//! 两种读法：
//! - [`replay`]：完整回放，保留消息原文和每个字段在文件里的位置，阅读视图用；
//! - [`scan_header`]：只取列表要用的元数据、标题和「有没有可恢复的消息」，工具结果、
//!   图片、思考这些大字段用 `IgnoredAny` 跳过，列表扫描用。

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader};
use std::marker::PhantomData;
use std::path::Path;

use indexmap::IndexMap;
use serde::de::value::MapAccessDeserializer;
use serde::de::{Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::{Map, Value};

use super::super::utils::{JsonlSpan, LineSpans};

/// 官方回放后跳过、也不算可恢复内容的注入消息前缀（初始环境上下文、hook 上下文）
pub(crate) const INJECTED_PREFIXES: [&str; 2] = ["<session_context>", "<hook_context>"];

/// 列表标题与可恢复判断只需要文本开头，超过这个字节数的部分不再保留
const TEXT_KEEP_BYTES: usize = 4096;

pub(crate) fn is_jsonl(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "jsonl")
}

/// 用户消息是否可恢复（官方 `isResumableMessageRecord`）：文本非空，且不以 `/`、`?`
/// 或注入前缀开头。只含工具结果（functionResponse）的消息没有文本，不算。
pub(crate) fn is_resumable_user_text(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && !text.starts_with('/')
        && !text.starts_with('?')
        && !INJECTED_PREFIXES.iter().any(|p| text.starts_with(p))
}

// ─── 元数据 ──────────────────────────────────────────────────────────────

/// 浅合并后的会话元数据（只取 cc-switch 用得到的键）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Meta {
    pub session_id: Option<String>,
    pub project_hash: Option<String>,
    pub start_time: Option<String>,
    pub last_updated: Option<String>,
    pub summary: Option<String>,
    /// `main` | `subagent`；很老的文件没有
    pub kind: Option<String>,
}

impl Meta {
    /// 官方加载结束时要求两者都是字符串，否则按旧格式整份解析
    pub fn is_complete(&self) -> bool {
        self.session_id.is_some() && self.project_hash.is_some()
    }

    pub fn is_subagent(&self) -> bool {
        self.kind.as_deref() == Some("subagent")
    }

    fn slots(&mut self) -> [(&'static str, &mut Option<String>); 6] {
        [
            ("sessionId", &mut self.session_id),
            ("projectHash", &mut self.project_hash),
            ("startTime", &mut self.start_time),
            ("lastUpdated", &mut self.last_updated),
            ("summary", &mut self.summary),
            ("kind", &mut self.kind),
        ]
    }

    /// 浅合并元数据行或 `$set`：出现的键覆盖旧值；值不是字符串时视为清除
    fn merge_object(&mut self, obj: &Map<String, Value>) {
        for (key, slot) in self.slots() {
            if let Some(value) = obj.get(key) {
                *slot = value.as_str().map(str::to_string);
            }
        }
    }

    fn merge_fast(&mut self, rec: &FastRecord) {
        let fields = [
            &rec.session_id,
            &rec.project_hash,
            &rec.start_time,
            &rec.last_updated,
            &rec.summary,
            &rec.kind,
        ];
        for ((_, slot), field) in self.slots().into_iter().zip(fields) {
            if let Some(value) = field {
                *slot = value.as_str().map(str::to_string);
            }
        }
    }
}

// ─── 有序映射上的回放操作（两种读法共用）──────────────────────────────────

/// `$rewindTo`：删掉该 id 及其后所有消息；id 不存在时清空（官方行为）
fn rewind<V>(msgs: &mut IndexMap<String, V>, id: &str) {
    match msgs.get_index_of(id) {
        Some(index) => msgs.truncate(index),
        None => msgs.clear(),
    }
}

/// `$patch.orderIds`：其中仍存在的 id 按给定顺序排到最后，其余保持原相对顺序在前
fn reorder<'a, V>(msgs: &mut IndexMap<String, V>, order: impl Iterator<Item = &'a str>) {
    let mut seen = HashSet::new();
    let ordered: Vec<String> = order
        .filter(|id| msgs.contains_key(*id) && seen.insert(*id))
        .map(str::to_string)
        .collect();
    let tail: HashSet<&str> = ordered.iter().map(String::as_str).collect();
    let mut moved = HashMap::with_capacity(ordered.len());
    let mut rebuilt = IndexMap::with_capacity(msgs.len());
    for (id, value) in msgs.drain(..) {
        if tail.contains(id.as_str()) {
            moved.insert(id, value);
        } else {
            rebuilt.insert(id, value);
        }
    }
    for id in ordered {
        if let Some(value) = moved.remove(&id) {
            rebuilt.insert(id, value);
        }
    }
    *msgs = rebuilt;
}

// ─── 完整回放 ────────────────────────────────────────────────────────────

/// 某个值在会话文件里的位置：JSONL 的一行 + 行内 JSON Pointer；旧格式整份 JSON 时
/// `span` 为 `None`，`pointer` 相对整个文件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Src {
    pub span: Option<JsonlSpan>,
    pub pointer: String,
}

/// 回放后的一条消息。
#[derive(Debug, Clone)]
pub(crate) struct ReplayedMessage {
    /// 应用过补丁的消息对象
    pub value: Value,
    /// 消息本身的位置：消息行为 `""`，`$set.messages[i]` 为 `/$set/messages/{i}`，
    /// 单行旧 JSON 为 `/messages/{i}`
    pub src: Src,
    /// `content` 被 `$patch` 替换过时，指向补丁行里的 content
    pub content_src: Option<Src>,
    /// 被 `$patch` 替换过结果的工具调用：call id → 补丁行里的 result
    pub tool_result_src: HashMap<String, Src>,
}

impl ReplayedMessage {
    fn new(value: Value, src: Src) -> Self {
        Self {
            value,
            src,
            content_src: None,
            tool_result_src: HashMap::new(),
        }
    }
}

/// 完整回放的结果。
#[derive(Debug, Default)]
pub(crate) struct Replayed {
    /// 阅读视图只用消息；元数据留给测试核对回放结果（列表走 [`scan_header`]）
    #[cfg_attr(not(test), allow(dead_code))]
    pub meta: Meta,
    pub messages: Vec<ReplayedMessage>,
}

/// 按官方算法回放整个会话文件（`.jsonl` 逐行回放，缺元数据时与 `.json` 一样整份解析）。
pub(crate) fn replay(path: &Path) -> Result<Replayed, String> {
    if is_jsonl(path) {
        let file = File::open(path).map_err(|e| format!("Failed to read session: {e}"))?;
        let mut lines = LineSpans::new(BufReader::new(file));
        let mut state = FullState::default();
        while let Some(line) = lines
            .next_line()
            .map_err(|e| format!("Failed to read session: {e}"))?
        {
            let bytes = line.bytes.trim_ascii();
            if bytes.is_empty() {
                continue;
            }
            // 坏行、写到一半的末行跳过
            let Ok(Value::Object(rec)) = serde_json::from_slice::<Value>(bytes) else {
                continue;
            };
            state.apply(line.span, rec);
        }
        if state.meta.is_complete() {
            return Ok(Replayed {
                meta: state.meta,
                messages: state.msgs.into_values().collect(),
            });
        }
    }
    replay_legacy(path)
}

/// 旧格式：整份 JSON。消息原样保留（不按 id 去重，与改版前的解析一致）。
fn replay_legacy(path: &Path) -> Result<Replayed, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read session: {e}"))?;
    let value: Value =
        serde_json::from_slice(&data).map_err(|e| format!("Failed to parse session JSON: {e}"))?;
    let Value::Object(mut obj) = value else {
        return Err("No messages array found".to_string());
    };
    let mut meta = Meta::default();
    meta.merge_object(&obj);
    let Some(Value::Array(messages)) = obj.remove("messages") else {
        return Err("No messages array found".to_string());
    };
    let messages = messages
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            ReplayedMessage::new(
                value,
                Src {
                    span: None,
                    pointer: format!("/messages/{i}"),
                },
            )
        })
        .collect();
    Ok(Replayed { meta, messages })
}

#[derive(Default)]
struct FullState {
    meta: Meta,
    msgs: IndexMap<String, ReplayedMessage>,
}

impl FullState {
    /// 一行记录，判定顺序与官方 `processLine` 一致
    fn apply(&mut self, span: JsonlSpan, mut rec: Map<String, Value>) {
        if let Some(Value::String(target)) = rec.get("$rewindTo") {
            rewind(&mut self.msgs, target);
            return;
        }
        if let Some(Value::Object(_)) = rec.get("$patch") {
            if let Some(Value::Object(patch)) = rec.remove("$patch") {
                self.apply_patch(span, patch);
            }
            return;
        }
        if !rec.contains_key("$patch") {
            if let Some(Value::String(id)) = rec.get("id") {
                let id = id.clone();
                self.insert(id, Value::Object(rec), span, String::new());
                return;
            }
        }
        if let Some(Value::Object(_)) = rec.get("$set") {
            if let Some(Value::Object(mut set)) = rec.remove("$set") {
                // 2026-10-01 之前的全量 checkpoint：清空后按数组重建
                if let Some(Value::Array(messages)) = set.remove("messages") {
                    self.msgs.clear();
                    self.insert_all(span, "/$set/messages", messages);
                }
                self.meta.merge_object(&set);
            }
            return;
        }
        let is_str = |key: &str| matches!(rec.get(key), Some(Value::String(_)));
        if is_str("sessionId") && is_str("projectHash") {
            let messages = rec.remove("messages");
            self.meta.merge_object(&rec);
            // 单行写成的旧 JSON
            if let Some(Value::Array(messages)) = messages {
                self.insert_all(span, "/messages", messages);
            }
        }
    }

    /// 已存在的 id 原位整条替换（之前的补丁来源随之作废），否则追加到末尾
    fn insert(&mut self, id: String, value: Value, span: JsonlSpan, pointer: String) {
        let src = Src {
            span: Some(span),
            pointer,
        };
        self.msgs.insert(id, ReplayedMessage::new(value, src));
    }

    fn insert_all(&mut self, span: JsonlSpan, prefix: &str, messages: Vec<Value>) {
        for (i, message) in messages.into_iter().enumerate() {
            let Value::Object(obj) = &message else {
                continue;
            };
            if obj.contains_key("$patch") {
                continue;
            }
            let Some(id) = obj.get("id").and_then(Value::as_str).map(str::to_string) else {
                continue;
            };
            self.insert(id, message, span, format!("{prefix}/{i}"));
        }
    }

    fn apply_patch(&mut self, span: JsonlSpan, mut patch: Map<String, Value>) {
        // 单条形式：顶层就是一条 MessagePatch（读取端支持，当前写入端不用）
        if patch.get("id").is_some_and(Value::is_string) {
            self.patch_message(span, "/$patch", &mut patch);
        }
        if let Some(Value::Array(updates)) = patch.get_mut("updates") {
            for (k, update) in updates.iter_mut().enumerate() {
                if let Value::Object(update) = update {
                    if update.get("id").is_some_and(Value::is_string) {
                        self.patch_message(span, &format!("/$patch/updates/{k}"), update);
                    }
                }
            }
        }
        if let Some(Value::Array(ids)) = patch.get("removeIds") {
            for id in ids.iter().filter_map(Value::as_str) {
                // shift_remove 保持其余消息的顺序
                self.msgs.shift_remove(id);
            }
        }
        if let Some(Value::Array(ids)) = patch.get("orderIds") {
            reorder(&mut self.msgs, ids.iter().filter_map(Value::as_str));
        }
    }

    /// MessagePatch：有 `content` 键就整体替换 content；gemini 消息已有 toolCalls 时，
    /// 按 id 找到对应调用替换 `result`（找不到的调用忽略，不会新增）。
    fn patch_message(&mut self, span: JsonlSpan, base: &str, patch: &mut Map<String, Value>) {
        let Some(id) = patch.get("id").and_then(Value::as_str).map(str::to_string) else {
            return;
        };
        let Some(message) = self.msgs.get_mut(&id) else {
            return;
        };
        if let Some(content) = patch.get_mut("content") {
            if let Some(obj) = message.value.as_object_mut() {
                obj.insert("content".to_string(), content.take());
            }
            message.content_src = Some(Src {
                span: Some(span),
                pointer: format!("{base}/content"),
            });
        }
        let is_gemini = message.value.get("type").and_then(Value::as_str) == Some("gemini");
        let Some(Value::Array(call_patches)) = patch.get_mut("toolCalls") else {
            return;
        };
        let Some(Value::Array(calls)) = message.value.get_mut("toolCalls") else {
            return;
        };
        if !is_gemini {
            return;
        }
        for (k, call_patch) in call_patches.iter_mut().enumerate() {
            let Some(call_patch) = call_patch.as_object_mut() else {
                continue;
            };
            let Some(call_id) = call_patch
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
            else {
                continue;
            };
            let Some(result) = call_patch.get_mut("result") else {
                continue;
            };
            let Some(call) = calls
                .iter_mut()
                .find(|call| call.get("id").and_then(Value::as_str) == Some(call_id.as_str()))
                .and_then(Value::as_object_mut)
            else {
                continue;
            };
            call.insert("result".to_string(), result.take());
            message.tool_result_src.insert(
                call_id,
                Src {
                    span: Some(span),
                    pointer: format!("{base}/toolCalls/{k}/result"),
                },
            );
        }
    }
}

// ─── 快速模式（列表扫描）─────────────────────────────────────────────────

/// 列表要用的会话摘要。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionHeader {
    pub meta: Meta,
    /// 第一次出现的可恢复用户消息：`displayContent` 的文本优先，其次 `content` 的 text
    /// 部件（之后被回退也保留，与官方 `firstUserMessage` 一致）；只保留开头一段
    pub first_user_text: Option<String>,
    /// 回放后是否还有可恢复的消息；官方列表隐藏没有的会话
    pub has_resumable: bool,
}

/// 快速读取会话摘要。读文件失败返回 `Err`（列表缓存会下轮重试），
/// 不是会话（解析不出 `sessionId`）返回 `Ok(None)`。
pub(crate) fn scan_header(path: &Path) -> io::Result<Option<SessionHeader>> {
    if is_jsonl(path) {
        let state = read_fast_jsonl(path)?;
        if state.meta.is_complete() {
            return Ok(Some(SessionHeader {
                has_resumable: state.msgs.values().any(|m| m.resumable),
                meta: state.meta,
                first_user_text: state.first_user_text,
            }));
        }
    }

    // 旧格式整份 JSON：消息不按 id 去重，逐条看
    let data = std::fs::read(path)?;
    let Ok(rec) = serde_json::from_slice::<FastRecord>(&data) else {
        return Ok(None);
    };
    let mut state = FastState::default();
    state.meta.merge_fast(&rec);
    if state.meta.session_id.is_none() {
        return Ok(None);
    }
    let mut has_resumable = false;
    if let Some(LooseSeq::Items(messages)) = rec.messages {
        for message in messages {
            if let Loose::Obj(message) = message {
                has_resumable |= state.observe(message).resumable;
            }
        }
    }
    Ok(Some(SessionHeader {
        meta: state.meta,
        first_user_text: state.first_user_text,
        has_resumable,
    }))
}

fn read_fast_jsonl(path: &Path) -> io::Result<FastState> {
    let file = File::open(path)?;
    let mut lines = LineSpans::new(BufReader::new(file));
    let mut state = FastState::default();
    while let Some(line) = lines.next_line()? {
        let bytes = line.bytes.trim_ascii();
        if bytes.is_empty() {
            continue;
        }
        let Ok(rec) = serde_json::from_slice::<FastRecord>(bytes) else {
            continue;
        };
        state.apply(rec);
    }
    Ok(state)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Gemini,
    Other,
}

#[derive(Debug, Clone, Copy)]
struct FastMsg {
    role: Role,
    /// gemini 消息有 toolCalls 或 thoughts
    has_extras: bool,
    resumable: bool,
}

fn is_resumable(role: Role, content: &PartsInfo, has_extras: bool) -> bool {
    match role {
        Role::User => content.has_text && is_resumable_user_text(&content.text),
        Role::Gemini => content.has_text || has_extras,
        Role::Other => false,
    }
}

#[derive(Default)]
struct FastState {
    meta: Meta,
    msgs: IndexMap<String, FastMsg>,
    first_user_text: Option<String>,
}

impl FastState {
    /// 与 [`FullState::apply`] 同一套判定顺序，只是不保留消息内容
    fn apply(&mut self, mut rec: FastRecord) {
        if let Some(LooseStr::Str(target)) = &rec.rewind_to {
            rewind(&mut self.msgs, target);
            return;
        }
        let has_patch_key = rec.patch.is_some();
        if let Some(Loose::Obj(patch)) = rec.patch.take() {
            self.apply_patch(patch);
            return;
        }
        if !has_patch_key && matches!(rec.id, Some(LooseStr::Str(_))) {
            self.insert(rec);
            return;
        }
        if let Some(Loose::Obj(set)) = rec.set.take() {
            let mut set = *set;
            if let Some(LooseSeq::Items(messages)) = set.messages.take() {
                self.msgs.clear();
                self.insert_all(messages);
            }
            self.meta.merge_fast(&set);
            return;
        }
        let is_str = |field: &Option<LooseStr>| matches!(field, Some(LooseStr::Str(_)));
        if is_str(&rec.session_id) && is_str(&rec.project_hash) {
            self.meta.merge_fast(&rec);
            if let Some(LooseSeq::Items(messages)) = rec.messages.take() {
                self.insert_all(messages);
            }
        }
    }

    fn insert_all(&mut self, messages: Vec<Loose<FastRecord>>) {
        for message in messages {
            if let Loose::Obj(message) = message {
                if message.patch.is_none() && matches!(message.id, Some(LooseStr::Str(_))) {
                    self.insert(message);
                }
            }
        }
    }

    fn insert(&mut self, mut rec: FastRecord) {
        let Some(LooseStr::Str(id)) = rec.id.take() else {
            return;
        };
        let message = self.observe(rec);
        self.msgs.insert(id, message);
    }

    /// 看一条消息：判断角色与可恢复性，记下第一条可恢复的用户消息
    fn observe(&mut self, rec: FastRecord) -> FastMsg {
        let role = match rec.msg_type.as_ref().and_then(LooseStr::as_str) {
            Some("user") => Role::User,
            Some("gemini") => Role::Gemini,
            _ => Role::Other,
        };
        let content = rec.content.unwrap_or_default();
        let count = |field: &Option<ArrayLen>| field.as_ref().map_or(0, |len| len.0);
        let has_extras = count(&rec.tool_calls) > 0 || count(&rec.thoughts) > 0;
        let resumable = is_resumable(role, &content, has_extras);
        if role == Role::User && resumable && self.first_user_text.is_none() {
            let display = rec
                .display_content
                .filter(|d| d.has_text && !d.text.trim().is_empty());
            self.first_user_text = Some(display.map_or(content.text, |d| d.text));
        }
        FastMsg {
            role,
            has_extras,
            resumable,
        }
    }

    fn apply_patch(&mut self, patch: FastPatch) {
        if let Some(LooseStr::Str(id)) = &patch.id {
            self.patch_content(id, patch.content);
        }
        if let Some(LooseSeq::Items(updates)) = patch.updates {
            for update in updates {
                if let Loose::Obj(FastPatchItem {
                    id: Some(LooseStr::Str(id)),
                    content,
                }) = update
                {
                    self.patch_content(&id, content);
                }
            }
        }
        if let Some(LooseSeq::Items(ids)) = &patch.remove_ids {
            for id in ids.iter().filter_map(LooseStr::as_str) {
                self.msgs.shift_remove(id);
            }
        }
        if let Some(LooseSeq::Items(ids)) = &patch.order_ids {
            reorder(&mut self.msgs, ids.iter().filter_map(LooseStr::as_str));
        }
    }

    /// 补丁替换了 content：重新判断可恢复性（工具结果的补丁不影响）
    fn patch_content(&mut self, id: &str, content: Option<PartsInfo>) {
        let (Some(message), Some(content)) = (self.msgs.get_mut(id), content) else {
            return;
        };
        message.resumable = is_resumable(message.role, &content, message.has_extras);
    }
}

// ─── 快速模式的宽松反序列化 ───────────────────────────────────────────────
//
// 官方逐个字段做 `typeof` 判断，类型不对的字段只是被忽略，不会让整行失效；
// 这里的类型都接受任意 JSON 值，只提取需要的部分。

/// 按 JSON 值的类型分别处理；没覆盖到的类型落到 `other()`。
trait Shape<'de>: Sized {
    fn other() -> Self;

    fn from_str(_value: &str) -> Self {
        Self::other()
    }

    fn from_bool(_value: bool) -> Self {
        Self::other()
    }

    fn from_map<A: MapAccess<'de>>(mut map: A) -> Result<Self, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(Self::other())
    }

    fn from_seq<A: SeqAccess<'de>>(mut seq: A) -> Result<Self, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Self::other())
    }
}

struct ShapeVisitor<T>(PhantomData<T>);

impl<'de, T: Shape<'de>> Visitor<'de> for ShapeVisitor<T> {
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<T, E> {
        Ok(T::from_bool(value))
    }

    fn visit_i64<E>(self, _: i64) -> Result<T, E> {
        Ok(T::other())
    }

    fn visit_u64<E>(self, _: u64) -> Result<T, E> {
        Ok(T::other())
    }

    fn visit_f64<E>(self, _: f64) -> Result<T, E> {
        Ok(T::other())
    }

    fn visit_str<E>(self, value: &str) -> Result<T, E> {
        Ok(T::from_str(value))
    }

    fn visit_unit<E>(self) -> Result<T, E> {
        Ok(T::other())
    }

    fn visit_none<E>(self) -> Result<T, E> {
        Ok(T::other())
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        T::from_map(map)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<T, A::Error> {
        T::from_seq(seq)
    }
}

macro_rules! deserialize_via_shape {
    ($($ty:ident $(<$param:ident>)?),*) => {$(
        impl<'de $(, $param: Deserialize<'de>)?> Deserialize<'de> for $ty $(<$param>)? {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                d.deserialize_any(ShapeVisitor(PhantomData))
            }
        }
    )*};
}

deserialize_via_shape!(
    LooseStr,
    LooseBool,
    ArrayLen,
    PartsInfo,
    PartElem,
    Loose<T>,
    LooseSeq<T>
);

/// 键存在（即使值为 null）时为 `Some`，用于区分「没有这个键」与「值为 null」
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// 只关心是不是字符串
#[derive(Debug, Clone, PartialEq, Eq)]
enum LooseStr {
    Str(String),
    Other,
}

impl LooseStr {
    fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(value) => Some(value),
            Self::Other => None,
        }
    }
}

impl Shape<'_> for LooseStr {
    fn other() -> Self {
        Self::Other
    }

    fn from_str(value: &str) -> Self {
        Self::Str(value.to_string())
    }
}

/// 只关心是不是 `true`
struct LooseBool(bool);

impl Shape<'_> for LooseBool {
    fn other() -> Self {
        Self(false)
    }

    fn from_bool(value: bool) -> Self {
        Self(value)
    }
}

/// 数组的元素个数；不是数组为 0
struct ArrayLen(usize);

impl<'de> Shape<'de> for ArrayLen {
    fn other() -> Self {
        Self(0)
    }

    fn from_seq<A: SeqAccess<'de>>(mut seq: A) -> Result<Self, A::Error> {
        let mut len = 0;
        while seq.next_element::<IgnoredAny>()?.is_some() {
            len += 1;
        }
        Ok(Self(len))
    }
}

/// 是对象时按 `T` 解析，否则为 `Other`
enum Loose<T> {
    Obj(T),
    Other,
}

impl<'de, T: Deserialize<'de>> Shape<'de> for Loose<T> {
    fn other() -> Self {
        Self::Other
    }

    fn from_map<A: MapAccess<'de>>(map: A) -> Result<Self, A::Error> {
        T::deserialize(MapAccessDeserializer::new(map)).map(Self::Obj)
    }
}

/// 是数组时逐个按 `T` 解析，否则为 `Other`
enum LooseSeq<T> {
    Items(Vec<T>),
    Other,
}

impl<'de, T: Deserialize<'de>> Shape<'de> for LooseSeq<T> {
    fn other() -> Self {
        Self::Other
    }

    fn from_seq<A: SeqAccess<'de>>(mut seq: A) -> Result<Self, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element::<T>()? {
            items.push(item);
        }
        Ok(Self::Items(items))
    }
}

/// `content` / `displayContent`（PartListUnion：字符串、单个 Part 或 Part 数组）的摘要
#[derive(Debug, Clone, Default)]
struct PartsInfo {
    /// 非 thought 的 text 拼接（不加分隔符，与官方 `partListUnionToString` 一致），
    /// 只保留开头 [`TEXT_KEEP_BYTES`] 字节
    text: String,
    /// 是否有非空白文本（含没保留的部分）
    has_text: bool,
}

impl PartsInfo {
    fn push_text(&mut self, text: &str) {
        if !text.trim().is_empty() {
            self.has_text = true;
        }
        let room = TEXT_KEEP_BYTES.saturating_sub(self.text.len());
        let mut cut = room.min(text.len());
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        self.text.push_str(&text[..cut]);
    }

    fn push_part(&mut self, part: PartElem) {
        if let PartElem::Text(text) = part {
            self.push_text(&text);
        }
    }
}

impl<'de> Shape<'de> for PartsInfo {
    fn other() -> Self {
        Self::default()
    }

    fn from_str(value: &str) -> Self {
        let mut info = Self::default();
        info.push_text(value);
        info
    }

    fn from_map<A: MapAccess<'de>>(map: A) -> Result<Self, A::Error> {
        let mut info = Self::default();
        info.push_part(PartElem::from_map(map)?);
        Ok(info)
    }

    fn from_seq<A: SeqAccess<'de>>(mut seq: A) -> Result<Self, A::Error> {
        let mut info = Self::default();
        while let Some(part) = seq.next_element::<PartElem>()? {
            info.push_part(part);
        }
        Ok(info)
    }
}

/// Part 数组的一个元素：字符串、`{text}`（非 thought）或其他
enum PartElem {
    Text(String),
    Other,
}

impl<'de> Shape<'de> for PartElem {
    fn other() -> Self {
        Self::Other
    }

    fn from_str(value: &str) -> Self {
        Self::Text(value.to_string())
    }

    fn from_map<A: MapAccess<'de>>(map: A) -> Result<Self, A::Error> {
        let part = FastPart::deserialize(MapAccessDeserializer::new(map))?;
        Ok(match part {
            FastPart {
                text: Some(LooseStr::Str(text)),
                thought,
            } if !thought.as_ref().is_some_and(|t| t.0) => Self::Text(text),
            _ => Self::Other,
        })
    }
}

#[derive(Deserialize)]
struct FastPart {
    #[serde(default)]
    text: Option<LooseStr>,
    #[serde(default)]
    thought: Option<LooseBool>,
}

/// 一行记录（也用于 `$set` 对象和 `messages[]` 里的消息）。只声明用得到的键，
/// 其余键（工具调用参数与结果、图片、思考正文……）由 serde 跳过。
#[derive(Deserialize, Default)]
struct FastRecord {
    #[serde(rename = "$rewindTo", default)]
    rewind_to: Option<LooseStr>,
    #[serde(rename = "$patch", default, deserialize_with = "present")]
    patch: Option<Loose<FastPatch>>,
    #[serde(rename = "$set", default)]
    set: Option<Loose<Box<FastRecord>>>,

    #[serde(default)]
    id: Option<LooseStr>,
    #[serde(rename = "type", default)]
    msg_type: Option<LooseStr>,
    #[serde(default)]
    content: Option<PartsInfo>,
    #[serde(rename = "displayContent", default)]
    display_content: Option<PartsInfo>,
    #[serde(rename = "toolCalls", default)]
    tool_calls: Option<ArrayLen>,
    #[serde(default)]
    thoughts: Option<ArrayLen>,

    #[serde(rename = "sessionId", default, deserialize_with = "present")]
    session_id: Option<LooseStr>,
    #[serde(rename = "projectHash", default, deserialize_with = "present")]
    project_hash: Option<LooseStr>,
    #[serde(rename = "startTime", default, deserialize_with = "present")]
    start_time: Option<LooseStr>,
    #[serde(rename = "lastUpdated", default, deserialize_with = "present")]
    last_updated: Option<LooseStr>,
    #[serde(default, deserialize_with = "present")]
    summary: Option<LooseStr>,
    #[serde(default, deserialize_with = "present")]
    kind: Option<LooseStr>,
    #[serde(default)]
    messages: Option<LooseSeq<Loose<FastRecord>>>,
}

#[derive(Deserialize, Default)]
struct FastPatch {
    #[serde(default)]
    id: Option<LooseStr>,
    #[serde(default, deserialize_with = "present")]
    content: Option<PartsInfo>,
    #[serde(default)]
    updates: Option<LooseSeq<Loose<FastPatchItem>>>,
    #[serde(rename = "removeIds", default)]
    remove_ids: Option<LooseSeq<LooseStr>>,
    #[serde(rename = "orderIds", default)]
    order_ids: Option<LooseSeq<LooseStr>>,
}

#[derive(Deserialize, Default)]
struct FastPatchItem {
    #[serde(default)]
    id: Option<LooseStr>,
    #[serde(default, deserialize_with = "present")]
    content: Option<PartsInfo>,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// 规格 8.1 的主会话样例（id 改成短名，base64 截断）
    pub(crate) const MAIN_SESSION: &str = r##"{"sessionId":"5f0c1a2b-3c4d-4e5f-8a9b-0c1d2e3f4a5b","projectHash":"b99de0b0c35a9c32a62938a8364f1136cdc5c34d4930f332dd728f54e825a380","startTime":"2026-10-07T08:30:12.001Z","lastUpdated":"2026-10-07T08:30:12.001Z","kind":"main"}
{"id":"m-ctx","timestamp":"2026-10-07T08:30:12.050Z","type":"user","content":[{"text":"<session_context>\nThis is the Gemini CLI. ...\n</session_context>"}]}
{"$set":{"lastUpdated":"2026-10-07T08:30:12.051Z"}}
{"id":"m-u1","timestamp":"2026-10-07T08:30:20.100Z","type":"user","content":[{"text":"看一下 @logo.png，再把 README 标题改成 My App"},{"text":"\n--- Content from referenced files ---"},{"inlineData":{"mimeType":"image/png","data":"iVBORw0KGgoAAAANSUhEUgAA"}},{"text":"\n--- End of content ---"}],"displayContent":[{"text":"看一下 @logo.png，再把 README 标题改成 My App"}]}
{"$set":{"lastUpdated":"2026-10-07T08:30:20.101Z"}}
{"id":"m-g1","timestamp":"2026-10-07T08:30:25.500Z","type":"gemini","content":"这是一个蓝色圆形 logo。我先读 README。","thoughts":[{"subject":"Planning the edit","description":"Read README.md before replacing the title.","timestamp":"2026-10-07T08:30:24.900Z"}],"tokens":{"input":12034,"output":38,"cached":8192,"thoughts":120,"tool":0,"total":12192},"model":"gemini-3-pro-preview"}
{"$set":{"lastUpdated":"2026-10-07T08:30:25.501Z"}}
{"id":"m-g1","timestamp":"2026-10-07T08:30:25.500Z","type":"gemini","content":"这是一个蓝色圆形 logo。我先读 README。","thoughts":[{"subject":"Planning the edit","description":"Read README.md before replacing the title.","timestamp":"2026-10-07T08:30:24.900Z"}],"tokens":{"input":12034,"output":38,"cached":8192,"thoughts":120,"tool":0,"total":12192},"model":"gemini-3-pro-preview","toolCalls":[{"id":"read_file__read_file_1791366625500_0","name":"read_file","args":{"file_path":"README.md"},"result":[{"functionResponse":{"id":"read_file__read_file_1791366625500_0","name":"read_file","response":{"output":"# my-app\n\nA demo project.\n"}}}],"status":"success","timestamp":"2026-10-07T08:30:26.010Z","resultDisplay":"","description":"README.md","displayName":"ReadFile","renderOutputAsMarkdown":true}]}
{"id":"m-u2","timestamp":"2026-10-07T08:30:26.020Z","type":"user","content":[{"functionResponse":{"id":"read_file__read_file_1791366625500_0","name":"read_file","response":{"output":"# my-app\n\nA demo project.\n"}}}]}
{"$set":{"lastUpdated":"2026-10-07T08:30:26.021Z"}}
{"id":"m-g2","timestamp":"2026-10-07T08:30:29.300Z","type":"gemini","content":"","thoughts":[],"tokens":{"input":13510,"output":96,"cached":12000,"thoughts":210,"tool":0,"total":13816},"model":"gemini-3-pro-preview"}
{"$set":{"lastUpdated":"2026-10-07T08:30:29.301Z"}}
{"id":"m-g2","timestamp":"2026-10-07T08:30:29.300Z","type":"gemini","content":"","thoughts":[],"tokens":{"input":13510,"output":96,"cached":12000,"thoughts":210,"tool":0,"total":13816},"model":"gemini-3-pro-preview","toolCalls":[{"id":"replace__replace_1791366629300_0","name":"replace","args":{"file_path":"/Users/alice/code/my-app/README.md","old_string":"# my-app","new_string":"# My App"},"result":[{"functionResponse":{"id":"replace__replace_1791366629300_0","name":"replace","response":{"output":"Successfully modified file: /Users/alice/code/my-app/README.md (1 replacements)."}}}],"status":"success","timestamp":"2026-10-07T08:30:31.000Z","resultDisplay":{"fileDiff":"--- README.md\n+++ README.md\n@@ -1 +1 @@\n-# my-app\n+# My App\n","fileName":"README.md","filePath":"/Users/alice/code/my-app/README.md","originalContent":"# my-app\n\nA demo project.\n","newContent":"# My App\n\nA demo project.\n"},"description":"README.md: # my-app => # My App","displayName":"Edit","renderOutputAsMarkdown":true}]}
{"id":"m-u3","timestamp":"2026-10-07T08:30:31.010Z","type":"user","content":[{"functionResponse":{"id":"replace__replace_1791366629300_0","name":"replace","response":{"output":"Successfully modified file: /Users/alice/code/my-app/README.md (1 replacements)."}}}]}
{"$set":{"lastUpdated":"2026-10-07T08:30:31.011Z"}}
{"id":"m-g3","timestamp":"2026-10-07T08:30:33.000Z","type":"gemini","content":"README 标题已改为 My App。","thoughts":[],"tokens":{"input":13900,"output":24,"cached":13400,"thoughts":0,"tool":0,"total":13924},"model":"gemini-3-pro-preview"}
{"$set":{"lastUpdated":"2026-10-07T08:30:33.001Z"}}
{"id":"m-i1","timestamp":"2026-10-07T08:31:00.000Z","type":"info","content":"（示例：UI 提示文本）"}
{"$set":{"lastUpdated":"2026-10-07T08:31:00.001Z"}}
{"id":"m-u4","timestamp":"2026-10-07T08:31:10.000Z","type":"user","content":[{"text":"再加一个 LICENSE 文件"}]}
{"$set":{"lastUpdated":"2026-10-07T08:31:10.001Z"}}
{"id":"m-g4","timestamp":"2026-10-07T08:31:13.000Z","type":"gemini","content":"好的，先确认要用哪种许可证？","thoughts":[],"tokens":{"input":14100,"output":40,"cached":13800,"thoughts":64,"tool":0,"total":14204},"model":"gemini-3-pro-preview"}
{"$set":{"lastUpdated":"2026-10-07T08:31:13.001Z"}}
{"$rewindTo":"m-u4"}
{"id":"m-u5","timestamp":"2026-10-07T08:32:00.000Z","type":"user","content":[{"text":"让 codebase_investigator 看一下项目结构"}]}
{"$set":{"lastUpdated":"2026-10-07T08:32:00.001Z"}}
{"id":"m-g5","timestamp":"2026-10-07T08:32:03.000Z","type":"gemini","content":"","thoughts":[],"tokens":{"input":14050,"output":60,"cached":13800,"thoughts":150,"tool":0,"total":14260},"model":"gemini-3-pro-preview"}
{"$set":{"lastUpdated":"2026-10-07T08:32:03.001Z"}}
{"id":"m-g5","timestamp":"2026-10-07T08:32:03.000Z","type":"gemini","content":"","thoughts":[],"tokens":{"input":14050,"output":60,"cached":13800,"thoughts":150,"tool":0,"total":14260},"model":"gemini-3-pro-preview","toolCalls":[{"id":"codebase_investigator__codebase_investigator_1791366723000_0","name":"codebase_investigator","args":{"objective":"Describe the project structure"},"result":[{"functionResponse":{"id":"codebase_investigator__codebase_investigator_1791366723000_0","name":"codebase_investigator","response":{"output":"src/ 下分为 core 与 cli 两个包……"}}}],"status":"success","timestamp":"2026-10-07T08:32:40.000Z","agentId":"a1b2c3d4-0000-4000-8000-000000000001","resultDisplay":{"isSubagentProgress":true,"agentName":"codebase_investigator","recentActivity":[{"id":"act-1","type":"tool_call","content":"list_directory","status":"completed"}],"state":"completed","result":"src/ 下分为 core 与 cli 两个包……"},"description":"Describe the project structure","displayName":"Codebase Investigator Agent","renderOutputAsMarkdown":true}]}
{"id":"m-u6","timestamp":"2026-10-07T08:32:40.010Z","type":"user","content":[{"functionResponse":{"id":"codebase_investigator__codebase_investigator_1791366723000_0","name":"codebase_investigator","response":{"output":"src/ 下分为 core 与 cli 两个包……"}}}]}
{"$set":{"lastUpdated":"2026-10-07T08:32:40.011Z"}}
{"id":"m-g6","timestamp":"2026-10-07T08:32:45.000Z","type":"gemini","content":"项目分为 core 与 cli 两个包……","thoughts":[],"tokens":{"input":15200,"output":180,"cached":14000,"thoughts":0,"tool":0,"total":15380},"model":"gemini-3-pro-preview"}
{"$set":{"lastUpdated":"2026-10-07T08:32:45.001Z"}}
{"$patch":{"updates":[{"id":"m-u2","content":[{"functionResponse":{"id":"read_file__read_file_1791366625500_0","name":"read_file","response":{"output":"<tool_output_masked>README.md, 3 lines</tool_output_masked>"}}}]},{"id":"m-g1","toolCalls":[{"id":"read_file__read_file_1791366625500_0","result":[{"functionResponse":{"id":"read_file__read_file_1791366625500_0","name":"read_file","response":{"output":"<tool_output_masked>README.md, 3 lines</tool_output_masked>"}}}]}]}],"removeIds":["m-i1"]}}
{"$set":{"lastUpdated":"2026-10-07T08:33:00.001Z"}}
{"$set":{"summary":"修改 README 标题并分析项目结构","memoryScratchpad":{"version":1,"toolSequence":["read_file","replace","codebase_investigator"],"touchedPaths":["README.md"],"validationStatus":"unknown"}}}
"##;

    /// 规格 8.2 的子代理样例
    pub(crate) const SUBAGENT_SESSION: &str = r##"{"sessionId":"a1b2c3d4-0000-4000-8000-000000000001","projectHash":"b99de0b0c35a9c32a62938a8364f1136cdc5c34d4930f332dd728f54e825a380","startTime":"2026-10-07T08:32:04.000Z","lastUpdated":"2026-10-07T08:32:04.000Z","kind":"subagent","directories":["/Users/alice/code/my-app"]}
{"id":"s-u1","timestamp":"2026-10-07T08:32:04.100Z","type":"user","content":[{"text":"Describe the project structure"}]}
{"$set":{"lastUpdated":"2026-10-07T08:32:04.101Z"}}
{"id":"s-g1","timestamp":"2026-10-07T08:32:08.000Z","type":"gemini","content":"","thoughts":[],"tokens":{"input":5200,"output":44,"cached":0,"thoughts":90,"tool":0,"total":5334},"model":"gemini-3-flash-preview"}
{"$set":{"lastUpdated":"2026-10-07T08:32:08.001Z"}}
{"id":"s-g1","timestamp":"2026-10-07T08:32:08.000Z","type":"gemini","content":"","thoughts":[],"tokens":{"input":5200,"output":44,"cached":0,"thoughts":90,"tool":0,"total":5334},"model":"gemini-3-flash-preview","toolCalls":[{"id":"list_directory__list_directory_1791366728000_0","name":"list_directory","args":{"dir_path":"src"},"result":[{"functionResponse":{"id":"list_directory__list_directory_1791366728000_0","name":"list_directory","response":{"output":"[DIR] cli\n[DIR] core"}}}],"status":"success","timestamp":"2026-10-07T08:32:09.000Z","resultDisplay":"Listed 2 item(s).","description":"src","displayName":"ReadFolder","renderOutputAsMarkdown":true}]}
{"id":"s-u2","timestamp":"2026-10-07T08:32:09.010Z","type":"user","content":[{"functionResponse":{"id":"list_directory__list_directory_1791366728000_0","name":"list_directory","response":{"output":"[DIR] cli\n[DIR] core"}}}]}
{"$set":{"lastUpdated":"2026-10-07T08:32:09.011Z"}}
{"id":"s-g2","timestamp":"2026-10-07T08:32:39.000Z","type":"gemini","content":"src/ 下分为 core 与 cli 两个包……","thoughts":[],"tokens":{"input":6100,"output":210,"cached":4096,"thoughts":0,"tool":0,"total":6310},"model":"gemini-3-flash-preview"}
{"$set":{"lastUpdated":"2026-10-07T08:32:39.001Z"}}
{"$set":{"summary":"src/ 下分为 core 与 cli 两个包……"}}
"##;

    pub(crate) const PROJECT_HASH: &str =
        "b99de0b0c35a9c32a62938a8364f1136cdc5c34d4930f332dd728f54e825a380";

    fn write(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn lines(values: &[Value]) -> String {
        values.iter().map(|v| format!("{v}\n")).collect()
    }

    fn ids(replayed: &Replayed) -> Vec<&str> {
        replayed
            .messages
            .iter()
            .map(|m| m.value["id"].as_str().unwrap())
            .collect()
    }

    fn meta_line() -> Value {
        json!({ "sessionId": "s1", "projectHash": "h1", "startTime": "2026-10-07T00:00:00Z",
                "lastUpdated": "2026-10-07T00:00:00Z", "kind": "main" })
    }

    fn user(id: &str, text: &str) -> Value {
        json!({ "id": id, "type": "user", "content": [{ "text": text }] })
    }

    fn gemini(id: &str, text: &str) -> Value {
        json!({ "id": id, "type": "gemini", "content": text, "thoughts": [],
                "tokens": { "input": 10, "output": 1, "cached": 0, "thoughts": 0, "tool": 0, "total": 11 } })
    }

    /// 快速模式与完整回放得到同样的元数据和消息顺序
    fn assert_fast_agrees(path: &Path, replayed: &Replayed) {
        let header = scan_header(path).unwrap().expect("header");
        assert_eq!(header.meta, replayed.meta);
        if is_jsonl(path) && replayed.messages.iter().all(|m| m.src.span.is_some()) {
            let state = read_fast_jsonl(path).unwrap();
            let fast: Vec<&str> = state.msgs.keys().map(String::as_str).collect();
            assert_eq!(fast, ids(replayed));
        }
    }

    #[test]
    fn replays_spec_main_session() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "session-a.jsonl", MAIN_SESSION);
        let replayed = replay(&path).unwrap();

        assert_eq!(
            ids(&replayed),
            [
                "m-ctx", "m-u1", "m-g1", "m-u2", "m-g2", "m-u3", "m-g3", "m-u5", "m-g5", "m-u6",
                "m-g6"
            ]
        );
        assert_eq!(
            replayed.meta.session_id.as_deref(),
            Some("5f0c1a2b-3c4d-4e5f-8a9b-0c1d2e3f4a5b")
        );
        assert_eq!(
            replayed.meta.last_updated.as_deref(),
            Some("2026-10-07T08:33:00.001Z")
        );
        assert_eq!(
            replayed.meta.summary.as_deref(),
            Some("修改 README 标题并分析项目结构")
        );
        assert_eq!(replayed.meta.kind.as_deref(), Some("main"));

        // m-g1 取第 8 行（带 toolCalls）的版本，位置仍在 m-u1 之后；read_file 结果被补丁替换
        let g1 = &replayed.messages[2];
        let masked = "<tool_output_masked>README.md, 3 lines</tool_output_masked>";
        assert_eq!(
            g1.value["toolCalls"][0]["result"][0]["functionResponse"]["response"]["output"],
            masked
        );
        let patch_src = &g1.tool_result_src["read_file__read_file_1791366625500_0"];
        assert_eq!(patch_src.pointer, "/$patch/updates/1/toolCalls/0/result");
        assert_eq!(g1.src.pointer, "");
        let u2 = &replayed.messages[3];
        assert_eq!(
            u2.value["content"][0]["functionResponse"]["response"]["output"],
            masked
        );
        assert_eq!(
            u2.content_src.as_ref().unwrap().pointer,
            "/$patch/updates/0/content"
        );
        // 补丁行的区间指向文件里的 `$patch` 那一行
        let span = patch_src.span.unwrap();
        let raw = &MAIN_SESSION.as_bytes()[span.offset as usize..][..span.len as usize];
        assert!(raw.starts_with(b"{\"$patch\""));

        assert_fast_agrees(&path, &replayed);
        let header = scan_header(&path).unwrap().unwrap();
        assert!(header.has_resumable);
        // m-ctx 以 <session_context> 开头，跳过；m-u1 取 displayContent
        assert_eq!(
            header.first_user_text.as_deref(),
            Some("看一下 @logo.png，再把 README 标题改成 My App")
        );
    }

    #[test]
    fn rewind_to_unknown_id_clears_all_messages() {
        let dir = tempfile::tempdir().unwrap();
        let text = lines(&[
            meta_line(),
            user("u1", "hello"),
            gemini("g1", "hi"),
            json!({ "$rewindTo": "missing" }),
            user("u2", "again"),
        ]);
        let path = write(dir.path(), "s.jsonl", &text);
        let replayed = replay(&path).unwrap();
        assert_eq!(ids(&replayed), ["u2"]);
        assert_fast_agrees(&path, &replayed);
        // 第一条可恢复的用户消息被回退后仍作标题
        let header = scan_header(&path).unwrap().unwrap();
        assert_eq!(header.first_user_text.as_deref(), Some("hello"));
    }

    #[test]
    fn patch_order_ids_moves_listed_messages_to_the_end() {
        let dir = tempfile::tempdir().unwrap();
        let text = lines(&[
            meta_line(),
            user("a", "1"),
            gemini("b", "2"),
            user("c", "3"),
            gemini("d", "4"),
            json!({ "$patch": { "orderIds": ["d", "missing", "b", "d"] } }),
            // 删除后再出现排到末尾；已存在的 id 原位替换
            json!({ "$patch": { "removeIds": ["a"] } }),
            user("a", "1 again"),
            user("c", "3 rewritten"),
        ]);
        let path = write(dir.path(), "s.jsonl", &text);
        let replayed = replay(&path).unwrap();
        assert_eq!(ids(&replayed), ["c", "d", "b", "a"]);
        assert_eq!(
            replayed.messages[0].value["content"][0]["text"],
            "3 rewritten"
        );
        assert_fast_agrees(&path, &replayed);
    }

    /// 对应官方测试 "should support legacy $set: { messages } checkpoints alongside new
    /// $patch and $rewindTo records"
    #[test]
    fn legacy_set_messages_checkpoint_mixes_with_patch_and_rewind() {
        let dir = tempfile::tempdir().unwrap();
        let text = lines(&[
            meta_line(),
            user("u1", "first"),
            gemini("g1", "one"),
            user("u2", "second"),
            // 全量 checkpoint：清空后按数组重建（u2 被压缩掉）
            json!({ "$set": { "messages": [user("u1", "first"), gemini("g1", "one (checkpoint)")],
                              "lastUpdated": "2026-10-07T01:00:00Z" } }),
            user("u3", "third"),
            gemini("g3", "three"),
            json!({ "$patch": { "updates": [{ "id": "g1", "content": [{ "text": "one (patched)" }] }] } }),
            json!({ "$rewindTo": "g3" }),
        ]);
        let path = write(dir.path(), "s.jsonl", &text);
        let replayed = replay(&path).unwrap();
        assert_eq!(ids(&replayed), ["u1", "g1", "u3"]);
        let g1 = &replayed.messages[1];
        assert_eq!(g1.value["content"][0]["text"], "one (patched)");
        assert_eq!(g1.src.pointer, "/$set/messages/1");
        assert_eq!(
            g1.content_src.as_ref().unwrap().pointer,
            "/$patch/updates/0/content"
        );
        assert_eq!(
            replayed.meta.last_updated.as_deref(),
            Some("2026-10-07T01:00:00Z")
        );
        assert_fast_agrees(&path, &replayed);
    }

    #[test]
    fn half_written_last_line_and_bad_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let mut text = lines(&[meta_line(), user("u1", "hello")]);
        text.push_str("not json\n\n");
        text.push_str(&lines(&[gemini("g1", "hi")]));
        text.push_str(r#"{"id":"g2","type":"gemini","content":"cut of"#);
        let path = write(dir.path(), "s.jsonl", &text);
        let replayed = replay(&path).unwrap();
        assert_eq!(ids(&replayed), ["u1", "g1"]);
        assert_fast_agrees(&path, &replayed);
    }

    /// 旧 `.json` 恢复两次时，同一个 `.jsonl` 里有两条元数据行和整批重复的消息
    #[test]
    fn repeated_migration_blocks_replay_to_the_same_messages() {
        let dir = tempfile::tempdir().unwrap();
        let block = [
            json!({ "sessionId": "s1", "projectHash": "h1", "startTime": "2026-03-20T09:15:02Z",
                    "lastUpdated": "2026-03-20T09:20:41Z", "kind": "main" }),
            user("o-u1", "hello"),
            gemini("o-g1", "Hi!"),
            json!({ "$set": { "sessionId": "s1" } }),
        ];
        let mut all = block.to_vec();
        all.extend(block.iter().cloned());
        all.push(user("o-u2", "after resume"));
        let path = write(dir.path(), "session-x.jsonl", &lines(&all));
        let replayed = replay(&path).unwrap();
        assert_eq!(ids(&replayed), ["o-u1", "o-g1", "o-u2"]);
        assert_eq!(
            replayed.meta.last_updated.as_deref(),
            Some("2026-03-20T09:20:41Z")
        );
        assert_fast_agrees(&path, &replayed);
    }

    /// 元数据缺 projectHash 的 `.jsonl` 和 `.json` 一样整份按旧格式解析
    #[test]
    fn falls_back_to_legacy_json_without_complete_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = json!({
            "sessionId": "old", "startTime": "2026-03-20T09:15:02Z",
            "messages": [ user("o-u1", "hello"), { "type": "info", "content": "no id" } ]
        });
        let pretty = serde_json::to_string_pretty(&legacy).unwrap();
        for name in ["session-a.json", "session-a.jsonl"] {
            let path = write(dir.path(), name, &pretty);
            let replayed = replay(&path).unwrap();
            assert_eq!(replayed.meta.session_id.as_deref(), Some("old"));
            assert_eq!(replayed.messages.len(), 2);
            assert_eq!(replayed.messages[1].src.pointer, "/messages/1");
            assert!(replayed.messages[1].src.span.is_none());

            let header = scan_header(&path).unwrap().unwrap();
            assert!(header.has_resumable);
            assert_eq!(header.first_user_text.as_deref(), Some("hello"));
        }

        let path = write(dir.path(), "session-b.jsonl", "{\"id\":\"u1\"}\n");
        assert!(replay(&path).is_err());
        assert_eq!(scan_header(&path).unwrap(), None);
    }

    /// 单行写成的旧 JSON：元数据行里带 messages
    #[test]
    fn single_line_legacy_json_inside_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let mut line = meta_line();
        line["messages"] = json!([user("u1", "hello"), gemini("g1", "hi")]);
        let path = write(dir.path(), "s.jsonl", &lines(&[line]));
        let replayed = replay(&path).unwrap();
        assert_eq!(ids(&replayed), ["u1", "g1"]);
        assert_eq!(replayed.messages[1].src.pointer, "/messages/1");
        assert_fast_agrees(&path, &replayed);
    }

    #[test]
    fn header_tracks_resumable_messages_and_display_content() {
        let dir = tempfile::tempdir().unwrap();
        // 只有注入上下文、斜杠命令和提示：没有可恢复的消息
        let text = lines(&[
            meta_line(),
            user("ctx", "<session_context>\nenv\n</session_context>"),
            user("cmd", "  /help"),
            json!({ "id": "i1", "type": "info", "content": "Authenticated" }),
        ]);
        let path = write(dir.path(), "a.jsonl", &text);
        let header = scan_header(&path).unwrap().unwrap();
        assert!(!header.has_resumable);
        assert_eq!(header.first_user_text, None);

        // content 是 Part 数组；displayContent 优先；thought 部件不算文本
        let text = lines(&[
            meta_line(),
            json!({ "id": "tool", "type": "user", "content": [{ "functionResponse": { "id": "c1", "name": "x", "response": {} } }] }),
            json!({ "id": "u1", "type": "user",
                    "content": [{ "text": "fix " }, { "text": "it", "thought": false }, { "text": "secret", "thought": true }, "!"],
                    "displayContent": [{ "text": "fix @a.ts" }] }),
        ]);
        let path = write(dir.path(), "b.jsonl", &text);
        let header = scan_header(&path).unwrap().unwrap();
        assert!(header.has_resumable);
        assert_eq!(header.first_user_text.as_deref(), Some("fix @a.ts"));

        // displayContent 为空时取 content 的 text 部件（不加分隔符）
        let text = lines(&[
            meta_line(),
            json!({ "id": "u1", "type": "user",
                    "content": [{ "text": "fix " }, { "text": "it", "thought": true }, "!"],
                    "displayContent": [{ "text": "  " }] }),
        ]);
        let path = write(dir.path(), "c.jsonl", &text);
        let header = scan_header(&path).unwrap().unwrap();
        assert_eq!(header.first_user_text.as_deref(), Some("fix !"));

        // gemini 消息只有思考也算可恢复；补丁把它的内容清空不影响
        let text = lines(&[
            meta_line(),
            json!({ "id": "g1", "type": "gemini", "content": "", "thoughts": [{ "subject": "s" }] }),
            json!({ "$patch": { "updates": [{ "id": "g1", "content": null }] } }),
        ]);
        let path = write(dir.path(), "d.jsonl", &text);
        assert!(scan_header(&path).unwrap().unwrap().has_resumable);

        // 唯一的用户消息被补丁改成工具结果后不再可恢复
        let text = lines(&[
            meta_line(),
            user("u1", "hello"),
            json!({ "$patch": { "id": "u1", "content": [{ "functionResponse": {} }] } }),
        ]);
        let path = write(dir.path(), "e.jsonl", &text);
        assert!(!scan_header(&path).unwrap().unwrap().has_resumable);
    }

    #[test]
    fn unexpected_field_types_do_not_invalidate_the_line() {
        let dir = tempfile::tempdir().unwrap();
        let text = lines(&[
            json!({ "sessionId": "s1", "projectHash": "h1", "kind": 3, "summary": null, "directories": "x" }),
            json!({ "id": "u1", "type": "user", "content": 42, "toolCalls": "nope", "thoughts": {} }),
            json!({ "id": "u2", "type": "user", "content": { "text": "single part" } }),
            json!({ "$patch": null, "id": "u3", "type": "user", "content": "ignored: has $patch key" }),
            json!({ "$set": { "summary": ["x"], "lastUpdated": "2026-10-07T00:00:00Z" } }),
        ]);
        let path = write(dir.path(), "s.jsonl", &text);
        let replayed = replay(&path).unwrap();
        assert_eq!(ids(&replayed), ["u1", "u2"]);
        assert_eq!(replayed.meta.kind, None);
        assert_eq!(replayed.meta.summary, None);
        assert_fast_agrees(&path, &replayed);
        let header = scan_header(&path).unwrap().unwrap();
        assert_eq!(header.first_user_text.as_deref(), Some("single part"));
    }

    #[test]
    fn subagent_sample_reports_kind_and_summary() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "agent.jsonl", SUBAGENT_SESSION);
        let header = scan_header(&path).unwrap().unwrap();
        assert!(header.meta.is_subagent());
        assert_eq!(header.meta.project_hash.as_deref(), Some(PROJECT_HASH));
        let replayed = replay(&path).unwrap();
        assert_eq!(ids(&replayed), ["s-u1", "s-g1", "s-u2", "s-g2"]);
        assert_fast_agrees(&path, &replayed);
    }
}
