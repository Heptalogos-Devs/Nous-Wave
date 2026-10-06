// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

export interface Ref {
  kind: string;
  value: string;
}
interface Evidence {
  reference?: Ref;
  supportRole: string;
}
type Requirement = "REQUIRED" | "PREFERRED" | "OPTIONAL" | "FORBIDDEN";
export interface ConsumerPolicy {
  consumerId: string;
  revision: string;
  memory: Requirement;
  runtime: Requirement;
  resource: Requirement;
  maxItems: number;
  maxTextBytes: number;
  materialize: boolean;
}
export interface Segment {
  segmentId: string;
  text: string;
  semanticRole: string;
  sourceRefs: Ref[];
  evidence: Evidence[];
  authority: string;
  stability: string;
  sourceRevision?: string;
}
export interface Degradation {
  code: string;
  detail: string;
}
export interface Projection {
  projectionId: string;
  consumerId: string;
  sourceRuntimeRevision: bigint;
  segments: Segment[];
  degradation: Degradation[];
}
export interface Cursor {
  trackId: string;
  epochId: string;
  revision: number;
}
export interface ContextPatch {
  kind: "RESET" | "APPEND";
  cursor: Cursor;
  projection: Projection;
}
export function refKey(ref: Ref): string {
  return ref.kind + "\0" + ref.value;
}
