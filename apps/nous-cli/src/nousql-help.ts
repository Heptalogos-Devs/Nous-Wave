// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

/** Standalone, daemon-free semantic guidance for Agent callers. */
export const nousqlHelp = {
  title: "NousQL Agent guide",
  manual: "docs/agent/NOUSQL.md",
  concepts: {
    subject:
      "The identity whose long-term cognition and current work Nous maintains.",
    cognition:
      "Memory, CognitiveSchema, Episode and Journal; the default return set.",
    tag: "A durable semantic concept with stable identity and revisable label/description; not just a category.",
    lexicalRef:
      "A stable Agent-readable reference; resolve names once and continue with the returned LexicalRef.",
    workContext:
      "A reusable current task with purpose, context text and real cognition/Entity/Tag anchors. Select it once and foreground it in a Session; subsequent queries consume the frozen task context.",
    cognitiveClock:
      "Subject logical time captured at query preparation; relative windows use this clock, not network timeout time.",
  },
  selectors: {
    text: "Strong recommendation: make confidently known referents explicit using concrete names and useful keywords for lexical, embedding and Entity/Tag activation. Preserve uncertainty; if unresolved, the original pronoun or short question remains a valid lexical+dense query.",
    concept: "#concept supplies semantic text without resolving a durable Tag.",
    entity:
      '@e("Alice") is an exact-resolved entity cue, not an exact cognition read.',
    tag: 'Recall relevant cognition @tag("reader reclamation") activates a durable Tag and directly recalls its attachments.',
    schema: '@schema("rule") supplies a schema cue.',
    resource: '@r("resource") supplies a resource cue.',
    exact:
      "@ref(mem:amber-lotus-cello-river) reads an exact identity; use returned LexicalRefs, never invent them.",
    object: '@object("host-ref") supplies an opaque Host object cue.',
    composition:
      "Mandatory unquoted Unicode intent plus optional $/@/# syntax islands; $prefer and $avoid express preferences; escape literal markers with backslash.",
  },
  time: {
    axes: {
      occurred: "When the source event happened.",
      observed: "When the Subject received the source evidence.",
      valid: "When the claim applies.",
      formed: "When cognition or a derived representation was formed.",
      recorded: "When the canonical revision was recorded.",
    },
    filter:
      "$time(occurred,within=30d); different axes intersect; at/from/to require timezone-bearing timestamps.",
    asOf: '$asof("2025-01-01T00:00:00Z") or $asof(ago=30d) selects what Authority knew at that cut.',
    history:
      "$history admits eligible prior cognition revisions; combine with $asof to exclude later knowledge.",
    distinction:
      "$time filters event/claim chronology; $asof changes the Authority view. They are independent.",
  },
  exploration: {
    direct: "Direct text and Tag recall need no graph diffusion.",
    associative:
      "$explore explicitly enables bounded diffusion from QueryActivation; the Agent does not select a physical algorithm.",
    enrichment:
      "Host configuration may optionally infer existing Tags or ephemeral concepts; it does not create Authority Tags.",
  },
  projection: {
    default: "memory,schema,episode,journal",
    root: "$return(cognition), $return(memory,schema), $return(evidence), $return(resource)",
    scope:
      "Root-only projection also limits exact targets; Evidence/Resource are not default top-level returns.",
  },
  shaping:
    "$limit(10), $effort(deep), $materialize, $diagnostics(full); all are root-only.",
  errors: {
    UNKNOWN_REFERENCE: "Resolve a known name or use a returned LexicalRef.",
    AMBIGUOUS_REFERENCE: "Choose a candidate LexicalRef and retry.",
    REFERENCE_TOMBSTONED:
      "Rediscover an active identity; do not guess a replacement.",
    UNRESOLVED_MACHINE_PLACEHOLDER:
      "Resolve a machine placeholder; ordinary pronouns remain valid.",
    STALE_CONTEXT: "Refresh current work/session references before retrying.",
    UNAVAILABLE:
      "Inspect capability/lane diagnostics; optional degradation is explicit.",
  },
  examples: [
    "How does Python free-threading work?",
    "Why did Simon Willison change his blogging practice later?",
    "Why did he change it later?",
    "她为什么修改这个决定？",
    "彼は後で何を変更しましたか？",
    'deployment approval @e("Alice")',
    'Recall relevant cognition @tag("active-reader reclamation")',
    "Recall migration decisions #migration",
    "deployment incidents $time(occurred,within=30d)",
    'policy state $time(valid,at="2026-05-01T00:00:00Z")',
    'evidence received $time(occurred,from="2026-01-01T00:00:00Z") $time(observed,to="2026-02-01T00:00:00Z")',
    'support status $asof("2025-01-01T00:00:00Z")',
    "support status $history",
    'Recall relevant cognition @tag("active-reader reclamation") $explore',
    "stale consumer problem $explore",
    "migration $return(memory,schema) $limit(10) $diagnostics(full)",
  ],
  workflow: [
    "nous subject use <actual-subject-id>",
    "nous session open",
    'nous context create --purpose "Investigate CPython free-threading" --text "Compare PEP 703, Python 3.13 and Python 3.14 extension compatibility."',
    "nous context foreground",
    "nous identity resolve --kind entity --name CPython",
    "nous context pin --entity <returned-entity-ref>",
    "nous query 'When did the support status change?'",
    "nous query 'Which extension limitations remained?'",
  ],
};
