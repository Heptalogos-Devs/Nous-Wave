// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use nous_core::*;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnresolvedQueryReference {
    pub span: String,
    pub start: usize,
    pub end: usize,
    pub kind: &'static str,
}

/// Shallow fail-fast checks for clear open references. Closure belongs to the caller.
pub fn unresolved_query_references(text: &str) -> Vec<UnresolvedQueryReference> {
    let mut spans = Vec::new();
    let mut add = |start: usize, end: usize, kind| {
        spans.push(UnresolvedQueryReference {
            span: text[start..end].into(),
            start,
            end,
            kind,
        });
    };
    for (start, _) in text.match_indices("<UNRESOLVED:") {
        let end = text[start..]
            .find('>')
            .map_or(text.len(), |n| start + n + 1);
        add(start, end, "unresolved_marker");
    }
    for (phrase, kind) in [
        ("上次那个", "temporal_deictic"),
        ("刚才那个", "temporal_deictic"),
        ("这个项目", "deictic"),
        ("那个项目", "deictic"),
        ("这个问题", "deictic"),
        ("那个问题", "deictic"),
        ("那个东西", "deictic"),
        ("这个东西", "deictic"),
        ("这件事", "deictic"),
        ("那件事", "deictic"),
    ] {
        for (start, _) in text.match_indices(phrase) {
            add(start, start + phrase.len(), kind);
        }
    }
    for pronoun in [
        "我们", "你们", "他们", "她们", "它们", "我", "你", "他", "她", "它",
    ] {
        for (start, _) in text.match_indices(pronoun) {
            let end = start + pronoun.len();
            if clear_chinese_pronoun(text, start, end) {
                add(start, end, "pronoun");
            }
        }
    }
    let lower = text.to_ascii_lowercase();
    let token_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\'';
    for (phrase, kind) in [
        ("the previous one", "temporal_deictic"),
        ("just now", "temporal_deictic"),
        ("last time", "temporal_deictic"),
        ("that project", "deictic"),
        ("this project", "deictic"),
        ("that memory", "deictic"),
        ("this memory", "deictic"),
        ("that problem", "deictic"),
        ("this problem", "deictic"),
    ] {
        for (start, _) in lower.match_indices(phrase) {
            let end = start + phrase.len();
            if !lower[..start].chars().next_back().is_some_and(token_char)
                && !lower[end..].chars().next().is_some_and(token_char)
            {
                add(start, end, kind);
            }
        }
    }
    let mut begin = None;
    for (offset, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if token_char(c) {
            begin.get_or_insert(offset);
        } else if let Some(start) = begin.take() {
            let token = &text[start..offset];
            if english_pronoun(token) {
                add(start, offset, "pronoun");
            }
        }
    }
    spans.sort_by_key(|span| (span.start, std::cmp::Reverse(span.end)));
    let mut result: Vec<UnresolvedQueryReference> = Vec::new();
    for span in spans {
        if !result
            .iter()
            .any(|previous| previous.start <= span.start && previous.end >= span.end)
        {
            result.push(span);
        }
    }
    result
}

fn english_pronoun(token: &str) -> bool {
    !matches!(token, "US" | "IT")
        && matches!(
            token.to_ascii_lowercase().as_str(),
            "i" | "me"
                | "my"
                | "mine"
                | "we"
                | "us"
                | "our"
                | "ours"
                | "you"
                | "your"
                | "yours"
                | "he"
                | "him"
                | "his"
                | "she"
                | "her"
                | "hers"
                | "it"
                | "its"
                | "they"
                | "them"
                | "their"
                | "theirs"
                | "i'm"
                | "we're"
                | "you're"
                | "he's"
                | "she's"
                | "it's"
                | "they're"
        )
}

fn clear_chinese_pronoun(text: &str, start: usize, end: usize) -> bool {
    let boundary = |c: char| {
        c.is_whitespace() || c.is_ascii_punctuation() || "，。！？、；：‘’“”（）【】".contains(c)
    };
    let left = text[..start].chars().next_back();
    let right = text[end..].chars().next();
    // Only a standalone token or a clear pronoun at a phrase boundary.
    let clear_left = left.is_none_or(boundary) || left.is_some_and(|c| "和与跟".contains(c));
    let clear_right = right.is_none_or(boundary)
        || [
            "和", "与", "在", "的", "昨天", "最近", "上次", "刚才", "怎么", "如何",
        ]
        .iter()
        .any(|word| text[end..].starts_with(word));
    clear_left && clear_right
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
            || !query.expression.targets.is_empty()
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
            "上次那个问题",
            "她最近怎么了",
            "<UNRESOLVED:person>",
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
