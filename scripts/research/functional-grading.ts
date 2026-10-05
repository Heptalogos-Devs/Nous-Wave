interface Link {
  occurrence_id?: string | null;
  ref_kind?: string;
  ref_value?: string;
  target_ref_kind?: string;
  target_ref?: string;
  support_kind?: string;
  support_ref?: string;
}
export interface SourceInspection {
  memories: { memory: string; revision: string; supports: Link[] }[];
  episodes: {
    episode_id: string;
    episode_revision_id: string;
    members: Link[];
  }[];
  journals: {
    journal_id: string;
    journal_revision_id: string;
    sources: Link[];
  }[];
  schemas: {
    schema_id: string;
    schema_revision_id: string;
    supports: Link[];
  }[];
  associations: {
    revoked_at?: string | null;
    supports: Link[];
    from_ref_kind: string;
    from_ref: string;
    to_ref_kind: string;
    to_ref: string;
  }[];
}
export interface Receipts {
  subjects: Record<
    string,
    { events: Record<string, { occurrenceId: string }> }
  >;
}
export function sourceIndex(inspection: SourceInspection, receipts: Receipts) {
  const roots = new Map<string, Set<string>>();
  const edges = new Map<string, Set<string>>();
  for (const [scenario, subject] of Object.entries(receipts.subjects))
    for (const [event, receipt] of Object.entries(subject.events))
      roots.set(
        `occurrence:${receipt.occurrenceId}`,
        new Set([`${scenario}:${event}`]),
      );
  const link = (from: string, to: string) => {
    const targets = edges.get(from) ?? new Set<string>();
    targets.add(to);
    edges.set(from, targets);
  };
  const supports = (from: string, values: Link[]) => {
    for (const value of values) {
      if (value.occurrence_id) link(from, `occurrence:${value.occurrence_id}`);
      const kind =
        value.ref_kind ?? value.target_ref_kind ?? value.support_kind;
      const id = value.ref_value ?? value.target_ref ?? value.support_ref;
      if (kind && id && kind !== "evidence") link(from, `${kind}:${id}`);
    }
  };
  for (const value of inspection.episodes) {
    const revision = `episode_revision:${value.episode_revision_id}`;
    supports(revision, value.members);
    link(`episode:${value.episode_id}`, revision);
  }
  for (const value of inspection.journals) {
    const revision = `journal_revision:${value.journal_revision_id}`;
    supports(revision, value.sources ?? []);
    link(`journal:${value.journal_id}`, revision);
  }
  for (const value of inspection.memories) {
    const revision = `memory_revision:${value.revision}`;
    supports(revision, value.supports);
    link(`memory:${value.memory}`, revision);
  }
  for (const value of inspection.schemas ?? []) {
    const revision = `cognitive_schema_revision:${value.schema_revision_id}`;
    supports(revision, value.supports ?? []);
    link(`cognitive_schema:${value.schema_id}`, revision);
  }
  for (const value of inspection.associations) {
    if (value.revoked_at) continue;
    for (const [kind, id] of [
      [value.from_ref_kind, value.from_ref],
      [value.to_ref_kind, value.to_ref],
    ])
      if (kind === "tag") supports(`${kind}:${id}`, value.supports);
  }
  return (reference: string) => {
    const found = new Set<string>();
    const visited = new Set<string>();
    const pending = [reference];
    while (pending.length) {
      const node = pending.pop()!;
      if (visited.has(node)) continue;
      visited.add(node);
      for (const root of roots.get(node) ?? []) found.add(root);
      pending.push(...(edges.get(node) ?? []));
    }
    return found;
  };
}
export interface Hit {
  reference: { kind: string; id: string };
  match_evidence: { explanation?: string | null; final_rank: number };
}
export function supportedRoute(hit: Hit, minimumHops: number) {
  if (!hit.match_evidence.explanation) return false;
  const explanation = JSON.parse(hit.match_evidence.explanation) as {
    topologywave?: {
      activated_route?: string[];
      route_evidence?: {
        from: { kind: string; id: string };
        to: { kind: string; id: string };
        flow: number;
        support: { provenance_root?: string | null; polarity: string }[];
      }[];
    };
  };
  const readout = explanation.topologywave;
  const path = readout?.activated_route;
  const evidence = readout?.route_evidence;
  if (
    !path ||
    !evidence ||
    path.length - 1 < minimumHops ||
    evidence.length !== path.length - 1 ||
    path.at(-1) !== `${hit.reference.kind}:${hit.reference.id}`
  )
    return false;
  return evidence.every(
    (edge, index) =>
      `${edge.from.kind}:${edge.from.id}` === path[index] &&
      `${edge.to.kind}:${edge.to.id}` === path[index + 1] &&
      edge.flow > 0 &&
      edge.support.length > 0 &&
      edge.support.every(
        (support) =>
          !!support.provenance_root && support.polarity === "positive",
      ),
  );
}
