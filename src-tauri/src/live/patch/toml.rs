//! TOML 补丁器（Codex、Grok Build 的 `config.toml`），基于 `toml_edit`：注释、空行、
//! 键的写法和表的位置都原样保留，只改补丁点名的键。

use std::path::Path;

use toml_edit::{DocumentMut, Value};

use super::{decode_utf8, line_column, KeyPath, LivePatch, LiveWriteError};

/// 改一份已经解析好的 TOML 文档。实现了它的补丁可以直接交给引擎：解析、改、再输出。
pub trait TomlDocPatch {
    fn apply_to(&self, path: &Path, doc: &mut DocumentMut) -> Result<(), LiveWriteError>;
}

impl<T: TomlDocPatch> LivePatch for T {
    fn apply(&self, path: &Path, pre: Option<&[u8]>) -> Result<Vec<u8>, LiveWriteError> {
        let mut doc = parse(path, pre)?;
        self.apply_to(path, &mut doc)?;
        Ok(doc.to_string().into_bytes())
    }
}

/// 在同一份文档上依次应用几个补丁（比如先应用编辑器里的改动，再换关键字段）。
pub struct TomlSteps<'a>(pub Vec<&'a dyn TomlDocPatch>);

impl TomlDocPatch for TomlSteps<'_> {
    fn apply_to(&self, path: &Path, doc: &mut DocumentMut) -> Result<(), LiveWriteError> {
        self.0.iter().try_for_each(|step| step.apply_to(path, doc))
    }
}

/// 解析写前内容；文件不存在时是空文档。
pub fn parse(path: &Path, pre: Option<&[u8]>) -> Result<DocumentMut, LiveWriteError> {
    let Some(bytes) = pre else {
        return Ok(DocumentMut::new());
    };
    let text = decode_utf8(path, bytes)?;
    text.parse::<DocumentMut>().map_err(|err| {
        let (line, column) = line_column(text, err.span().map_or(0, |span| span.start));
        LiveWriteError::Parse {
            path: path.to_path_buf(),
            line,
            column,
            message: err.message().to_string(),
        }
    })
}

/// 值的写法，不带两侧的空白和行尾注释。
pub fn value_text(value: &Value) -> String {
    let mut value = value.clone();
    value.decor_mut().clear();
    value.to_string()
}

/// 值相同就算相同，不看两侧的空白和行尾注释。
pub fn same_value(left: &Value, right: &Value) -> bool {
    value_text(left) == value_text(right)
}

/// 路径上的这一段应该是表，却不是。
pub fn shape_error(path: &Path, segments: &[String]) -> LiveWriteError {
    LiveWriteError::Shape {
        path: path.to_path_buf(),
        key_path: KeyPath(segments.to_vec()),
        expected: "表",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_files_are_refused_with_the_line_number() {
        let err = TomlSteps(Vec::new())
            .apply(Path::new("c.toml"), Some(b"a = 1\nb = \n"))
            .expect_err("must refuse");
        match err {
            LiveWriteError::Parse { line, .. } => assert_eq!(line, 2),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn missing_file_parses_as_an_empty_document() {
        let out = TomlSteps(Vec::new())
            .apply(Path::new("c.toml"), None)
            .expect("empty doc");
        assert!(out.is_empty());
    }

    #[test]
    fn same_value_ignores_whitespace_and_comments() {
        let doc: DocumentMut = "a = \"x\"   # note\nb = \"x\"\nc = \"y\"\n"
            .parse()
            .expect("doc");
        let get = |key: &str| doc[key].as_value().expect("value").clone();
        assert!(same_value(&get("a"), &get("b")));
        assert!(!same_value(&get("a"), &get("c")));
    }
}
