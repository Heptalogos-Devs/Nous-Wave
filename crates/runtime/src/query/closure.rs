use nous_core::*;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnresolvedQueryReference {
    pub span: String,
    pub start: usize,
    pub end: usize,
    pub kind: &'static str,
}

/// This guard detects open references; it never guesses their referents.
pub fn unresolved_query_references(text: &str) -> Vec<UnresolvedQueryReference> {
    let mut spans = Vec::new();
    let phrases = [
        ("上次那个", "temporal_deictic"),
        ("刚才那个", "temporal_deictic"),
        ("这个项目", "deictic"),
        ("那个项目", "deictic"),
        ("这件事", "deictic"),
        ("那件事", "deictic"),
        ("这个", "deictic"),
        ("那个", "deictic"),
        ("这些", "deictic"),
        ("那些", "deictic"),
        ("这里", "deictic"),
        ("那里", "deictic"),
        ("我们", "pronoun"),
        ("你们", "pronoun"),
        ("他们", "pronoun"),
        ("她们", "pronoun"),
        ("它们", "pronoun"),
        ("我", "pronoun"),
        ("你", "pronoun"),
        ("他", "pronoun"),
        ("她", "pronoun"),
        ("它", "pronoun"),
    ];
    let compounds = [
        "自我",
        "本我",
        "超我",
        "忘我",
        "无我",
        "其他",
        "其它",
        "他山之石",
    ];
    for (phrase, kind) in phrases {
        for (start, _) in text.match_indices(phrase) {
            let end = start + phrase.len();
            if spans
                .iter()
                .any(|span: &UnresolvedQueryReference| span.start <= start && span.end >= end)
            {
                continue;
            }
            let compound = compounds.iter().any(|compound| {
                text.match_indices(compound)
                    .any(|(offset, _)| offset <= start && offset + compound.len() >= end)
            });
            if compound {
                continue;
            }
            if phrase.chars().count() == 1 && !pronoun_context(text, start, end) {
                continue;
            }
            spans.push(UnresolvedQueryReference {
                span: phrase.into(),
                start,
                end,
                kind,
            });
        }
    }
    let lower_text = text.to_ascii_lowercase();
    for phrase in ["the previous one", "last time", "just now"] {
        for (start, _) in lower_text.match_indices(phrase) {
            let end = start + phrase.len();
            if lower_text[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric())
                || lower_text[end..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric())
            {
                continue;
            }
            spans.push(UnresolvedQueryReference {
                span: text[start..end].into(),
                start,
                end,
                kind: "temporal_deictic",
            });
        }
    }
    let mut start = None;
    for (offset, character) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if character.is_ascii_alphanumeric() || character == '_' || character == '\'' {
            start.get_or_insert(offset);
        } else if let Some(begin) = start.take() {
            let token = &text[begin..offset];
            let lower = token.to_ascii_lowercase();
            let kind = match lower.as_str() {
                "us" | "it" if matches!(token, "US" | "IT") => None,
                "i" | "me" | "my" | "mine" | "we" | "us" | "our" | "ours" | "you" | "your"
                | "yours" | "he" | "him" | "his" | "she" | "her" | "hers" | "it" | "its"
                | "they" | "them" | "their" | "theirs" | "i'm" | "we're" | "you're" | "he's"
                | "she's" | "it's" | "they're" => Some("pronoun"),
                "that" if relative_clause_connector(text, begin, offset) => None,
                "this" | "that" | "these" | "those" | "here" | "there" => Some("deictic"),
                _ => None,
            };
            if let Some(kind) = kind {
                spans.push(UnresolvedQueryReference {
                    span: token.into(),
                    start: begin,
                    end: offset,
                    kind,
                });
            }
        }
    }
    spans.sort_by_key(|span| (span.start, span.end));
    spans
}

// Distinguish a connective after a named cognitive noun from a demonstrative.
// This classifies syntax only; other open pronouns in the clause still fail.
fn relative_clause_connector(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start]
        .split_whitespace()
        .next_back()
        .unwrap_or("")
        .to_ascii_lowercase();
    let after = text[end..]
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        before.as_str(),
        "experience"
            | "experiences"
            | "memory"
            | "memories"
            | "event"
            | "events"
            | "decision"
            | "decisions"
            | "policy"
            | "policies"
            | "rule"
            | "rules"
            | "procedure"
            | "procedures"
            | "schema"
            | "schemas"
            | "routine"
            | "routines"
    ) && matches!(
        after.as_str(),
        "informed"
            | "caused"
            | "changed"
            | "replaced"
            | "explains"
            | "explained"
            | "records"
            | "recorded"
            | "requires"
            | "required"
            | "contains"
            | "contained"
            | "links"
            | "linked"
            | "supports"
            | "supported"
    )
}

fn pronoun_context(text: &str, start: usize, end: usize) -> bool {
    let left = text[..start].chars().next_back();
    let right = text[end..].chars().next();
    let boundary = |c: char| {
        c.is_whitespace() || c.is_ascii_punctuation() || "，。！？、；：‘’“”（）【】".contains(c)
    };
    let isolated = left.is_none_or(boundary) && right.is_none_or(boundary);
    let left_context = left.is_some_and(|c| "与和给对问说是由让跟请向帮把为比告诉".contains(c));
    let right_context = [
        "昨天", "最近", "的", "和", "与", "说", "在", "是", "有", "会", "要", "开发", "讨论", "用",
        "怎么", "如何", "完成", "一起", "已经", "上次", "刚才", "不", "能", "应该", "可能", "想",
        "好吗",
    ]
    .iter()
    .any(|prefix| text[end..].starts_with(prefix));
    isolated || left_context || right_context
}

pub fn validate_query_closure(query: &CognitiveQuery) -> Result<()> {
    let spans: Vec<_> = query
        .scopes()
        .iter()
        .flat_map(|node| &node.cues)
        .filter_map(|cue| match cue {
            Cue::Text(text) => Some(&text.text),
            Cue::Example(text) => Some(&text.text),
            _ => None,
        })
        .flat_map(|text| unresolved_query_references(text))
        .collect();
    if spans.is_empty() {
        return Ok(());
    }
    Err(Error::Invalid(serde_json::json!({
        "code": "UNRESOLVED_QUERY_REFERENCE", "message": "Prepared Query requires closed referents", "details": spans,
    }).to_string()))
}

pub(super) fn validate_query_input(query: &CognitiveQuery) -> Result<()> {
    query.validate()?;
    if query.text_only_compatibility {
        if query.session.is_some()
            || query.work_context.is_some()
            || !query.situation.current_refs.is_empty()
            || !query.situation.current_objects.is_empty()
            || !query.situation.object_descriptions.is_empty()
            || query.situation.consumer.is_some()
            || query.exploration != ExplorationIntent::None
            || query.expression.operation != QueryOperation::Atom
            || query.expression.cues.len() != 1
            || !matches!(query.expression.cues.first(), Some(Cue::Text(_)))
            || !query.expression.preferences.is_empty()
            || query.expression.targets.iter().any(|target| {
                !matches!(
                    target,
                    QueryTarget::AnyRelevantCognition
                        | QueryTarget::Memory
                        | QueryTarget::Schema
                        | QueryTarget::Episode
                        | QueryTarget::Journal
                        | QueryTarget::Evidence
                )
            })
            || serde_json::to_value(&query.expression.constraints)
                .map_err(|error| Error::Invalid(error.to_string()))?
                != serde_json::to_value(QueryConstraints::default())
                    .map_err(|error| Error::Invalid(error.to_string()))?
            || query.capabilities.residual_sensing == RequirementStrength::Required
            || query.resources.synopsis_only
        {
            return Err(Error::Invalid(
                "text compatibility requires one standalone text cue without cognitive context"
                    .into(),
            ));
        }
    } else {
        validate_query_closure(query)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guard_detects_open_references_without_matching_compound_words() {
        for text in [
            "我和他在这个项目进展上如何",
            "那个东西最近怎么样",
            "我们上次讨论的事",
            "他昨天说的模型",
            "How did we progress on that project?",
            "the previous one",
            "Retrieve that memory",
            "Summarize my deployment policy",
            "Recall their previous decision",
            "Experience that informed us about Tide",
            "Memory about that project",
            "Experience that informed us about that project",
        ] {
            assert!(!unresolved_query_references(text).is_empty(), "{text}");
        }
        for text in [
            "Arsvine 与 Alice 开发 Nous Wave 项目的近期进展",
            "Alice 在 2026-10-04 讨论的 doubao-seed-2.0-mini 模型",
            "Arsvine 自我认知与其他认知",
            "Alice researches iteration and white noise",
            "Prior snapshot experience that informed the joint build-cache incident review.",
            "Memory that records the Tide approval policy.",
            "US deployment policies and IT procedures",
            "Alice 查询他莫昔芬的作用",
        ] {
            assert!(unresolved_query_references(text).is_empty(), "{text}");
        }
    }
}
