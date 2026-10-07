# NousQL: Agent manual

Nous stores and retrieves long-term cognition for a Subject. A Subject is the
person or autonomous identity whose knowledge and current work Nous maintains.
A plain quoted question is already a complete query. Identity selectors, time,
context and exploration are optional ways to make its meaning more precise.

## 1. Objects you need to know

- **Subject**: the identity whose cognition and operating context you query.
  Supply its ID explicitly or select it through the CLI.
- **Cognition**: four durable kinds returned by default: Memory, CognitiveSchema,
  Episode and Journal. A Memory states a supported fact or experience; a Schema
  describes a reusable pattern/rule; an Episode organizes an experience; a
  Journal gives a supported narrative over settled Episodes.
- **Tag**: a durable semantic concept with a stable identity and revisable label
  and description. A Tag can express a topic, rule, experience pattern or a
  sentence-like idea. Its meaning can be embedded for semantic retrieval.
- **LexicalRef**: a stable Agent-readable reference such as
  `mem:amber-lotus-cello-river`. Names discover identities; a returned LexicalRef
  lets you continue using that exact identity. Never invent a LexicalRef.
- **WorkContext**: a bounded current task or question, with a purpose and exact
  cognition references. It can enrich a query, but is not required for text recall.
- **CognitiveClock**: the Subject's logical clock captured when a query is
  prepared. A live Host may advance it with real time. Replay, simulation and
  tests may set it independently. Relative query time uses this clock.
- **Exploration/diffusion**: bounded propagation from query activation through
  cognition relations to find indirect connections. Request it with `$explore`;
  do not select a physical algorithm.

## 2. Start with plain text

Write a self-contained intent. No Tag, Entity or WorkContext is required.

```nousql
"How does Python free-threading work?"
```

Text queries use direct lexical and dense retrieval when their Serving families
and embedding capability are available. They do not require graph diffusion,
concept enrichment or a query concept model.

Avoid unresolved pronouns such as “her project” unless you first identify the
person and project. Prefer an explicit name, object and question.

## 3. Resolve identities when they matter

```nousql
@e("Alice") && "deployment approval"
```

```nousql
@tag("active-reader reclamation")
```

An Entity selector is a typed cue. A Tag selector resolves a durable concept and
recalls its direct attachments. Names must resolve uniquely; ambiguity never
turns into a vector guess. Use the chosen candidate's LexicalRef on retry.

```nousql
@ref(mem:amber-lotus-cello-river)
```

This example shows syntax only. Replace its illustrative reference with a real
returned reference. `@ref` is an exact read; result projection still applies.
Other selectors are cues, not a promise to return that object itself.

## 4. Semantic text is different from a Tag

```nousql
#migration
```

```nousql
@tag("KRaft migration")
```

`#migration` contributes semantic concept text without an identity lookup.
`@tag(...)` activates an existing durable Tag. Neither creates a Tag.
Optional Host enrichment may infer existing Tags or ephemeral hypotheses.
It does not authorize a persistent concept mutation.

## 5. Combine intent and preferences

Use explicit `&&`, `||` and parentheses. `&&` binds more tightly than `||`.

```nousql
(@e("Alice") && "approval") || "deployment rollback"
```

Hard constraints restrict eligible results. A `+` or `-` preference changes
ranking within eligible results; it does not make a prohibited result eligible.

```nousql
"deployment" +"rollback" -"draft" +recent(recorded)
```

A recent preference needs an explicit time axis. Bare `recent` is invalid.

## 6. Choose the right time axis

| Axis | Meaning |
| --- | --- |
| `occurred` | When the source event happened. |
| `observed` | When the Subject received the source evidence. |
| `valid` | When the claim applies. |
| `formed` | When cognition or a derived representation was formed. |
| `recorded` | When the canonical revision was recorded. |

```nousql
"deployment incidents" $time(occurred,within=30d)
```

`within=30d` is relative to the prepared Subject CognitiveClock. It is unrelated
to network timeouts, process wall-clock deadlines or retry durations.

```nousql
"policy state" $time(valid,at="2026-05-01T00:00:00Z")
```

Absolute timestamps must carry a timezone. Different axes can be combined;
all constraints must match the same eligible evidence, not different observations.

```nousql
"evidence received" $time(occurred,from="2026-01-01T00:00:00Z") $time(observed,to="2026-02-01T00:00:00Z")
```

An interval is start-inclusive and end-exclusive. Unknown times do not satisfy
known-time constraints. Each axis can appear once in one scope; repeating the
same axis there is an error. `within` cannot mix with `from`, `to` or `at`.

## 7. Ask what was known then

A time filter and an Authority view answer different questions. `$time` filters
chronology; `$asof` chooses the knowledge state used for retrieval.

```nousql
"free-threading support status" $asof("2025-01-01T00:00:00Z")
```

```nousql
"support status" $asof(ago=30d)
```

At that cut, names, canonical Tags, revision heads, relations and Serving assets
use the historical state. A future Tag rename, merge or association does not
change the past view. Current permission and purge fences still apply.

## 8. Ask how cognition changed

```nousql
"support status" $history
```

The default view admits effective heads. `$history` also admits eligible prior
cognition revisions as independent retrieval documents. To exclude later
knowledge while inspecting earlier changes, combine the two controls.

```nousql
"support status" $history $asof("2025-01-01T00:00:00Z")
```

Five-axis filters still constrain the returned revisions. `$asof` and `$history`
are root-only and apply to the complete query, including identity resolution.

## 9. Direct recall versus associative exploration

```nousql
@tag("active-reader reclamation")
```

This asks for direct Tag attachments. A Tag with no attachments does not
silently start graph diffusion.

```nousql
@tag("active-reader reclamation") $explore
```

```nousql
"stale consumer problem" $explore
```

These requests explicitly allow bounded associative recall. QueryActivation
unifies text, explicit Tags, optional inferred concepts and permitted context.
An unavailable optional lane is reported; it does not substitute future assets.

## 10. Choose the returned object kinds

Default results are Memory, Schema, Episode and Journal. Evidence and Resource
are explicit choices rather than default top-level results.

```nousql
"migration" $return(schema)
```

```nousql
"migration" $return(memory,schema)
```

```nousql
"evidence received" $return(evidence)
```

`$return(cognition)` explicitly names the default set. Projection is root-only
and also limits exact targets. It does not request a particular physical lane.

## 11. Shape the answer and inspect it

```nousql
"migration" $limit(10) $diagnostics(full)
```

```nousql
"migration" $materialize
```

Use `query prepare` to inspect canonical query, captured time, resolved context,
projection and capability requirements before execution. Preparation does not
invoke embedding or concept models. Full execution diagnostics show actual
activation, generations, degradation and the query concept model call count.

## 12. A complete Agent workflow

1. Select the Subject and write a self-contained plain-text intent.
2. Resolve Entity/Tag names only when identity matters.
3. On ambiguity, choose a returned candidate LexicalRef and retry.
4. Add the appropriate time axes when the question has chronology.
5. Use `$asof` for “what was known then”; `$history` for prior revisions.
6. Add `$explore` only when indirect associative recall is wanted.
7. Add `$return` only when a particular returned domain is needed.
8. Inspect with `query prepare`, then execute and inspect any degradation.
9. Report actual use of exact returned references with the response `query_id`.
10. Grant bounded concept maintenance only when persistent changes are intended.

CLI examples (the launcher supplies local instance discovery):

```sh
nous help nousql --json
nous query prepare '"deployment approval" $return(memory,schema)' --subject <subject-id> --json
nous query '"deployment incidents" $time(occurred,within=30d)' --subject <subject-id> --json
nous use <returned-revision-ref> --kind referenced --query-id <query-id> --event-id <stable-event-id> --occurred-at <timestamp> --subject <subject-id> --json
```

For a retry, preserve the UseEvent ID, timestamp and consumer identity. `presented`
means exposure only; `referenced`, `acted_on` and `result_supported` are meaningful use.
`result_refuted` is negative feedback, not supporting evidence. Use feedback can
request review; it cannot grant a model or commit a Tag maintenance proposal.

Explicit formation Tags use `nous form <occurrence-id> --tag <tag-ref>`. Tag/Association
mutation commands and maintenance grants are listed by `nous help`; use
`--request-file` for structured revisions, merge/split and bounded maintenance.

## 13. Recover from errors

| Code | Next action |
| --- | --- |
| `UNKNOWN_REFERENCE` | Resolve a known name or use a returned LexicalRef. |
| `AMBIGUOUS_REFERENCE` | Select one returned candidate; do not guess. |
| `REFERENCE_TOMBSTONED` | Rediscover the current active identity. |
| `UNRESOLVED_QUERY_REFERENCE` | Make pronouns, objects and time explicit. |
| `STALE_CONTEXT` | Refresh current WorkContext/session references. |
| `UNAVAILABLE` | Inspect capability and lane diagnostics before retrying. |

A Tag that did not exist at an as-of cut cannot be used as a past identity.
Future cognition references in current WorkContext are excluded from historical
activation; the current question and purpose remain available.

[Agent entry](README.md) · [Developer implementation reference](../reference/NOUSQL.md)
