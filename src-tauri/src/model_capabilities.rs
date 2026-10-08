/// Image-input capability used by Codex catalog generation.
///
/// `Unknown` is intentionally distinct from `Supported`: the Codex catalog
/// treats unknown models as image-capable (fail open).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageInputCapability {
    Supported,
    Unsupported,
    Unknown,
}

/// Resolve image-input capability from an explicit declaration first, then the
/// confirmed text-only model registry when the caller enables registry lookup.
pub(crate) fn resolve_image_input_capability(
    model: &str,
    declared_support: Option<bool>,
    use_confirmed_registry: bool,
) -> ImageInputCapability {
    match declared_support {
        Some(true) => ImageInputCapability::Supported,
        Some(false) => ImageInputCapability::Unsupported,
        None if use_confirmed_registry && is_confirmed_text_only_model(model) => {
            ImageInputCapability::Unsupported
        }
        None => ImageInputCapability::Unknown,
    }
}

/// Convert a catalog row's explicit modality list into the shared capability
/// representation, falling back to the text-only registry when omitted.
pub(crate) fn image_input_capability_from_modalities(
    model: &str,
    modalities: Option<&[String]>,
) -> ImageInputCapability {
    let declared_support = modalities.map(|items| {
        items
            .iter()
            .any(|item| item.trim().eq_ignore_ascii_case("image"))
    });
    resolve_image_input_capability(model, declared_support, true)
}

/// Models that CC Switch is willing to advertise to clients as text-only.
///
/// This registry is deliberately exact and fail-open. A new suffix is not
/// inherited automatically: it remains image-capable until its capability is
/// confirmed, preventing a future `-vision`/`-vl` variant from being blocked by
/// the Codex client before a request can reach the provider.
pub(crate) fn is_confirmed_text_only_model(model: &str) -> bool {
    let normalized = normalize_model_id(model);
    let tail = normalized.rsplit('/').next().unwrap_or(normalized.as_str());

    const CONFIRMED_TAILS: &[&str] = &[
        "ark-code-latest",
        "deepseek-chat",
        "deepseek-reasoner",
        // `deepseek-v4-flash` is intentionally absent: it is a legacy alias the
        // vendor still accepts and routes to the vision-capable `deepseek-flash`
        // (api-docs.deepseek.com/guides/vision), so it must fail open.
        // `deepseek-v4-pro` likewise stays out of this global registry: the
        // official API continues serving V4 Pro after September 14, 2026
        // (api-docs.deepseek.com), but hosted aliases can differ. First-party
        // presets declare text-only explicitly; unknown gateways fail open.
        "glm-5.1",
        // Exact rather than prefix matching: GLM visual models use a `v`
        // suffix (for example glm-5.2v), which must remain image-capable.
        "glm-5.2",
        "glm-5.3",
        "kat-coder",
        "kat-coder-pro",
        "kat-coder-pro v1",
        "kat-coder-pro v2",
        "kat-coder-pro-v1",
        "kat-coder-pro-v2",
        "ling-2.5-1t",
        // Ant Ling ships vision as separate `-VL` models (Ling-3.0-flash-VL);
        // Ling-2.6-1T is text-only (developer.ant-ling.com model docs).
        "ling-2.6-1t",
        "longcat-2.0",
        "longcat-flash-chat",
        "minimax-m2.7",
        "minimax-m2.7-highspeed",
        "mimo-v2.5-pro",
        "qwen3-coder-480b",
        "qwen3-coder-480b-a35b-instruct",
        "qwen3-coder-flash",
        "qwen3-coder-next",
        "qwen3-coder-plus",
        "step-3.5-flash",
        "step-3.5-flash-2603",
        "us.deepseek.r1-v1",
    ];

    CONFIRMED_TAILS.contains(&tail)
}

/// 模型 id 末尾的 1M 上下文标记（比较时已转成小写）。
pub(crate) const ONE_M_CONTEXT_MARKER: &str = "[1m]";

fn normalize_model_id(value: &str) -> String {
    let mut normalized = value
        .trim()
        .trim_start_matches("models/")
        .trim()
        .to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_suffix(ONE_M_CONTEXT_MARKER) {
        normalized = stripped.trim().to_string();
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpt_and_unknown_models_remain_unknown_without_declarations() {
        for model in ["gpt-5.4", "gpt-5.5", "gpt-5.6-sol", "custom-alias"] {
            assert_eq!(
                resolve_image_input_capability(model, None, true),
                ImageInputCapability::Unknown,
                "{model} must fail open"
            );
        }
    }

    #[test]
    fn confirmed_text_only_registry_normalizes_namespaces_and_context_markers() {
        assert!(is_confirmed_text_only_model("deepseek/deepseek-chat"));
        // v4-flash 是旧别名，路由到识图的 deepseek-flash；2026-09-14 起 v4-pro 也路由到
        // 识图的 V4.1 Flash。两者均已移出名单，必须 fail-open。
        assert!(!is_confirmed_text_only_model("deepseek/deepseek-v4-flash"));
        assert!(!is_confirmed_text_only_model("deepseek/deepseek-v4-pro"));
        assert!(is_confirmed_text_only_model("GLM-5.2[1M]"));
        assert!(is_confirmed_text_only_model("GLM-5.3[1M]"));
        assert!(is_confirmed_text_only_model("qwen/qwen3-coder-plus"));
        assert!(is_confirmed_text_only_model(
            "Qwen/Qwen3-Coder-480B-A35B-Instruct"
        ));
        assert!(is_confirmed_text_only_model("MiniMax-M2.7-Highspeed"));
        assert!(is_confirmed_text_only_model("step-3.5-flash-2603"));
        assert!(!is_confirmed_text_only_model("glm-5.2v"));
        assert!(!is_confirmed_text_only_model("glm-5.3v"));
    }

    #[test]
    fn unconfirmed_family_suffixes_fail_open() {
        for model in [
            "minimax-m2.7-vision",
            "qwen3-coder-ultra",
            "qwen3-coder-vl",
            "step-3.5-flash-vision",
        ] {
            assert!(
                !is_confirmed_text_only_model(model),
                "unconfirmed variant {model} must not be hard-gated"
            );
        }
    }

    #[test]
    fn explicit_capability_overrides_the_registry() {
        assert_eq!(
            resolve_image_input_capability("qwen3-coder-plus", Some(true), true),
            ImageInputCapability::Supported
        );
        assert_eq!(
            resolve_image_input_capability("gpt-5.4", Some(false), true),
            ImageInputCapability::Unsupported
        );
    }
}
