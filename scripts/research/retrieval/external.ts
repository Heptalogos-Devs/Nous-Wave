import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

type Turn = { role: string; content: string; has_answer?: boolean };
type LmeItem = {
  question_id: string;
  question_type: string;
  question: string;
  question_date: string;
  answer_session_ids: string[];
  haystack_dates: string[];
  haystack_session_ids: string[];
  haystack_sessions: Turn[][];
};
type LocTurn = {
  speaker: string;
  dia_id: string;
  text: string;
  blip_caption?: string;
};
type LocItem = {
  sample_id: string;
  conversation: Record<string, string | LocTurn[]>;
  qa: { question: string; category: number; evidence?: string[] }[];
};
type Event = {
  event_id: string;
  subject: string;
  occurred_at: string;
  observed_at: string;
  formed_at: string;
  recorded_at: string;
  valid_time: { start: string; end: null };
  session: string;
  entities: string[];
  tags: string[];
  relations: never[];
  renders: { text: string }[];
  source_locator: string;
};
type Oracle = Record<string, { grade: number; reason: string }>;
type Query = {
  query_id: string;
  suite: string;
  subject: string;
  category: string;
  text: string;
  as_of: string;
  original_question_date?: string;
  oracle: Oracle;
  session_oracle: string[];
  unresolved_evidence: string[];
  oracle_status: string;
  time_axis: null;
};
const sha = (bytes: string | Buffer) =>
  createHash("sha256").update(bytes).digest("hex");
const categories: Record<number, string> = {
  1: "multi_hop",
  2: "temporal",
  3: "open_domain",
  4: "single_hop",
  5: "adversarial",
};
function event(
  subject: string,
  key: string,
  date: string,
  session: string,
  speaker: string,
  text: string,
  locator: string,
): Event {
  return {
    event_id: `${subject}-${key}`,
    subject,
    occurred_at: date,
    observed_at: date,
    formed_at: date,
    recorded_at: date,
    valid_time: { start: date, end: null },
    session,
    entities: [speaker],
    tags: [],
    relations: [],
    renders: [{ text: `${date} · ${speaker}: ${text}` }],
    source_locator: locator,
  };
}
function lmeDate(value: string): string {
  const match =
    /^(\d{4})\/(\d{2})\/(\d{2}) \([A-Za-z]+\) (\d{2}):(\d{2})$/.exec(value);
  if (!match) throw new Error(`Invalid LongMemEval date: ${value}`);
  return new Date(
    `${match[1]}-${match[2]}-${match[3]}T${match[4]}:${match[5]}:00Z`,
  ).toISOString();
}
function locDate(value: string): string {
  const match = /^(\d{1,2}):(\d{2}) (am|pm) on (\d{1,2}) (\w+), (\d{4})$/i.exec(
    value,
  );
  if (!match) throw new Error(`Invalid LoCoMo date: ${value}`);
  const months = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
  ];
  const month = months.indexOf(match[5]!);
  if (month < 0) throw new Error("Unknown month");
  const hour =
    (Number(match[1]) % 12) + (match[3]!.toLowerCase() === "pm" ? 12 : 0);
  return new Date(
    Date.UTC(Number(match[6]), month, Number(match[4]), hour, Number(match[2])),
  ).toISOString();
}
function latest(dates: string[]): string {
  return new Date(
    Math.max(...dates.map((date) => Date.parse(date))) + 1000,
  ).toISOString();
}
function normalizeLme(item: LmeItem) {
  const subject = `lme-${item.question_id}`;
  if (
    item.haystack_sessions.length !== item.haystack_dates.length ||
    item.haystack_sessions.length !== item.haystack_session_ids.length
  )
    throw new Error("LongMemEval parallel session arrays disagree");
  const events: Event[] = [],
    oracle: Oracle = {};
  const order = item.haystack_dates
    .map((date, index) => ({ date: lmeDate(date), index }))
    .sort((a, b) => a.date.localeCompare(b.date) || a.index - b.index);
  for (const { date, index } of order) {
    const sid = item.haystack_session_ids[index]!;
    for (const [turnIndex, turn] of item.haystack_sessions[index]!.entries()) {
      const e = event(
        subject,
        `s${index + 1}-t${turnIndex + 1}`,
        date,
        `${subject}:${sid}`,
        turn.role,
        turn.content,
        `question:${item.question_id}/session:${sid}/turn:${turnIndex}`,
      );
      events.push(e);
      if (turn.has_answer) {
        if (!item.answer_session_ids.includes(sid))
          throw new Error("Answer turn outside official evidence sessions");
        oracle[e.event_id] = { grade: 1, reason: "official_has_answer" };
      }
    }
  }
  if (
    item.answer_session_ids.some(
      (sid) => !item.haystack_session_ids.includes(sid),
    )
  )
    throw new Error("Missing LongMemEval evidence session");
  const query: Query = {
    query_id: `${subject}-q001`,
    suite: "longmemeval-s",
    subject,
    category: item.question_id.endsWith("_abs")
      ? "abstention"
      : item.question_type,
    text: item.question,
    original_question_date: lmeDate(item.question_date),
    as_of: latest([
      lmeDate(item.question_date),
      ...order.map((value) => value.date),
    ]),
    oracle,
    session_oracle: item.answer_session_ids.map((sid) => `${subject}:${sid}`),
    unresolved_evidence: [],
    oracle_status: "official_turn_and_session_annotations",
    time_axis: null,
  };
  return { subject, events, queries: [query] };
}
function normalizeLoc(item: LocItem) {
  const subject = `loc-${item.sample_id}`;
  const sessions = Object.keys(item.conversation)
    .filter((key) => /^session_\d+$/.test(key))
    .sort((a, b) => Number(a.split("_")[1]) - Number(b.split("_")[1]));
  const events: Event[] = [],
    byId = new Map<string, Event>();
  for (const session of sessions) {
    const date = locDate(item.conversation[`${session}_date_time`] as string);
    for (const turn of item.conversation[session] as LocTurn[]) {
      if (byId.has(turn.dia_id))
        throw new Error("Duplicate LoCoMo dialogue ID");
      const text =
        turn.text +
        (turn.blip_caption
          ? `\nProvided image caption: ${turn.blip_caption}`
          : "");
      const e = event(
        subject,
        turn.dia_id,
        date,
        `${subject}:${session}`,
        turn.speaker,
        text,
        `sample:${item.sample_id}/${session}/dialogue:${turn.dia_id}`,
      );
      events.push(e);
      byId.set(turn.dia_id, e);
    }
  }
  const queries = item.qa.map((qa, index): Query => {
    const oracle: Oracle = {},
      unresolved: string[] = [],
      evidenceSessions = new Set<string>();
    for (const raw of qa.evidence ?? []) {
      // Normalize separators and padded numeric IDs; preserve unresolvable labels.
      const matches = [...raw.matchAll(/D:?(\d+):(\d+)/g)];
      if (!matches.length) {
        unresolved.push(raw);
        continue;
      }
      for (const match of matches) {
        const key = `D${Number(match[1])}:${Number(match[2])}`,
          e = byId.get(key);
        if (!e) {
          unresolved.push(key);
          continue;
        }
        oracle[e.event_id] = { grade: 1, reason: "official_dialogue_evidence" };
        evidenceSessions.add(e.session);
      }
    }
    return {
      query_id: `${subject}-q${String(index + 1).padStart(3, "0")}`,
      suite: "locomo",
      subject,
      category: categories[qa.category] ?? `category_${qa.category}`,
      text: qa.question,
      as_of: latest(events.map((e) => e.recorded_at)),
      oracle,
      session_oracle: [...evidenceSessions],
      unresolved_evidence: unresolved,
      oracle_status: unresolved.length
        ? "official_evidence_with_unresolved_labels"
        : "official_dialogue_evidence",
      time_axis: null,
    };
  });
  return { subject, events, queries };
}

export async function prepareExternal(
  suite: string,
  rawPath: string,
  preparedPath: string,
) {
  if (suite !== "longmemeval-s" && suite !== "locomo")
    throw new Error("Unsupported external suite");
  const root = resolve(preparedPath),
    rawFile = resolve(rawPath),
    ignored = resolve("data/research") + "/";
  if (!root.startsWith(ignored) || !rawFile.startsWith(ignored))
    throw new Error("External data must remain under ignored data/research");
  const sources = JSON.parse(
    await readFile("docs/research/corpus/external-memory-sources.json", "utf8"),
  ) as {
    sources: { suite: string; sha256: string; license: string; url: string }[];
  };
  const source = sources.sources.find((entry) => entry.suite === suite)!;
  const bytes = await readFile(rawFile);
  if (sha(bytes) !== source.sha256)
    throw new Error("Raw external data digest differs from frozen source");
  const raw = JSON.parse(bytes.toString("utf8")) as LmeItem[] | LocItem[];
  await mkdir(resolve(root, "scenarios"), { recursive: true });
  const files: { path: string; sha256: string; events: number }[] = [],
    queries: Query[] = [];
  let count = 0;
  const subjects = new Set<string>();
  for (const item of raw) {
    const normalized =
      suite === "longmemeval-s"
        ? normalizeLme(item as LmeItem)
        : normalizeLoc(item as LocItem);
    if (subjects.has(normalized.subject))
      throw new Error("Duplicate external Subject");
    subjects.add(normalized.subject);
    const body = JSON.stringify({
      scenario_id: normalized.subject,
      suite,
      events: normalized.events,
    });
    const path = `scenarios/${normalized.subject}.json`;
    await writeFile(resolve(root, path), body + "\n", { mode: 0o600 });
    files.push({
      path,
      sha256: sha(body + "\n"),
      events: normalized.events.length,
    });
    count += normalized.events.length;
    queries.push(...normalized.queries);
  }
  const queryBytes = JSON.stringify({ schema_version: 1, queries });
  await writeFile(resolve(root, "queries.json"), queryBytes + "\n", {
    mode: 0o600,
  });
  const audit = {
    suite,
    adapter_version: "external-memory-turn-v1",
    source,
    subjects: subjects.size,
    events: count,
    questions: queries.length,
    categories: Object.fromEntries(
      [...new Set(queries.map((q) => q.category))].map((category) => [
        category,
        queries.filter((q) => q.category === category).length,
      ]),
    ),
    unresolved_evidence: queries
      .filter((q) => q.unresolved_evidence.length)
      .map((q) => ({ query_id: q.query_id, labels: q.unresolved_evidence })),
    no_positive_turn_labels: queries
      .filter((q) => !Object.keys(q.oracle).length)
      .map((q) => q.query_id),
    question_date_before_last_session: queries
      .filter(
        (q) =>
          q.original_question_date &&
          Date.parse(q.original_question_date) < Date.parse(q.as_of) - 1000,
      )
      .map((q) => q.query_id),
    time_mapping:
      "source session timestamp interpreted as UTC; all turns retain same session time; controlled availability=max(question date,last session)+1s",
    status: "prepared_not_imported_not_benchmarked",
  };
  await writeFile(
    resolve(root, "manifest.json"),
    JSON.stringify(
      {
        schema_version: 1,
        suite,
        license: source.license,
        scenario_files: files,
        event_count: count,
        query_count: queries.length,
        queries_sha256: sha(queryBytes + "\n"),
        source_sha256: source.sha256,
        adapter_version: audit.adapter_version,
        status: audit.status,
      },
      null,
      2,
    ) + "\n",
    { mode: 0o600 },
  );
  await writeFile(
    resolve(root, "adapter-audit.json"),
    JSON.stringify(audit, null, 2) + "\n",
    { mode: 0o600 },
  );
  console.log(
    JSON.stringify({
      suite,
      subjects: subjects.size,
      events: count,
      queries: queries.length,
      unresolved_queries: audit.unresolved_evidence.length,
    }),
  );
}
