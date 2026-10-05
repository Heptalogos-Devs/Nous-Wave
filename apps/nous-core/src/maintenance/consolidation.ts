import { create } from "@bufbuild/protobuf";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  CommitLongitudinalConsolidationRequestSchema,
  ConsolidationMemoryContentSchema,
  ConsolidationSchemaContentSchema,
  ConsolidationResultRefSchema,
  LongitudinalConsolidationActionSchema,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/consolidation_pb.js";
import type { MaintenancePlan } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import {
  TemporalExtentSchema,
  type ProducerSignature,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import type { ConsolidationProposal } from "../model/schemas/consolidation.js";

type Action = ConsolidationProposal["actions"][number];
function invalid(message: string): never {
  throw new ConnectError(message, Code.InvalidArgument);
}
function time(
  value: Extract<Action, { action: "create_memory" }>["content"]["validTime"],
) {
  if (value.kind === "unknown") return create(TemporalExtentSchema);
  if (value.kind === "instant")
    return create(TemporalExtentSchema, {
      value: { case: "instant", value: timestampFromDate(new Date(value.at)) },
    });
  return create(TemporalExtentSchema, {
    value: {
      case: "interval",
      value: {
        start: value.start
          ? timestampFromDate(new Date(value.start))
          : undefined,
        end: value.end ? timestampFromDate(new Date(value.end)) : undefined,
      },
    },
  });
}
export function consolidationRequest(
  plan: MaintenancePlan,
  proposal: ConsolidationProposal,
  operationId: string,
  producer: ProducerSignature,
) {
  if (
    !plan.consolidationSource ||
    proposal.actions.length > plan.maxConsolidationActions
  )
    invalid("Consolidation source or action envelope is invalid");
  const supports = new Map(
    plan.supports.map((entry) => [entry.key, entry.support]),
  );
  const entities = new Map(
    plan.entities.map((entry) => [entry.key, entry.entityRef]),
  );
  const candidates = new Map(
    plan.candidates.map((entry) => [entry.key, entry.target]),
  );
  const members = new Map(
    plan.members.map((entry) => [entry.key, entry.occurrenceId]),
  );
  const selectedSupports = (keys: string[]) => {
    if (new Set(keys).size !== keys.length)
      invalid("Duplicate consolidation supports");
    return keys.map(
      (key) =>
        supports.get(key) ??
        invalid("Consolidation support key is outside the catalog"),
    );
  };
  const selectedEntities = (keys: string[]) => {
    if (new Set(keys).size !== keys.length)
      invalid("Duplicate consolidation entities");
    return keys.map(
      (key) =>
        entities.get(key) ??
        invalid("Consolidation entity key is outside the catalog"),
    );
  };
  const target = (key: string, kind: string) => {
    const value =
      candidates.get(key) ??
      invalid("Consolidation target is outside current context");
    if (value.reference?.kind !== kind)
      invalid("Consolidation target has the wrong domain");
    return value;
  };
  const memory = (
    content: Extract<Action, { action: "create_memory" }>["content"],
  ) =>
    create(ConsolidationMemoryContentSchema, {
      cognitiveRole: content.cognitiveRole,
      formationMode: content.formationMode,
      groundingOccurrenceId:
        content.groundingMemberKey === null
          ? undefined
          : (members.get(content.groundingMemberKey) ??
            invalid("Grounding key is outside the member catalog")),
      semanticRole: content.semanticRole,
      text: content.text,
      title: content.title ?? undefined,
      supports: selectedSupports(content.supportKeys),
      aboutness: selectedEntities(content.entityKeys),
      validTime: time(content.validTime),
      epistemicClass: content.epistemicClass,
    });
  const schema = (
    content: Extract<Action, { action: "create_schema" }>["content"],
  ) =>
    create(ConsolidationSchemaContentSchema, {
      title: content.title ?? undefined,
      structuralClaim: content.structuralClaim,
      applicabilityScope: {
        description: content.applicability,
        aboutness: selectedEntities(content.entityKeys),
        tags: [],
        validTime: time(content.validTime),
      },
      boundaryDefinition: content.boundaryDefinition,
      formationKind: content.formationKind,
      evidenceLinks: content.evidence.map((link) => ({
        role: link.role,
        support: selectedSupports([link.supportKey])[0],
      })),
    });
  const endpoint = (
    value: Extract<Action, { action: "link_relation" }>["from"],
    index: number,
  ) => {
    if (value.kind === "candidate")
      return create(ConsolidationResultRefSchema, {
        target: {
          case: "reference",
          value: target(value.key, "memory_revision").reference!,
        },
      });
    if (value.index >= index)
      invalid("Consolidation relation requires an earlier action");
    const prior = proposal.actions[value.index];
    if (prior?.action !== "create_memory" && prior?.action !== "revise_memory")
      invalid("Strong Memory relations require Memory action endpoints");
    return create(ConsolidationResultRefSchema, {
      target: { case: "actionIndex", value: value.index },
    });
  };
  const actions = proposal.actions.map((action, index) => {
    switch (action.action) {
      case "skip":
        return create(LongitudinalConsolidationActionSchema, {
          action: { case: "skip", value: { reason: action.reason } },
        });
      case "create_memory":
        return create(LongitudinalConsolidationActionSchema, {
          action: {
            case: "createMemory",
            value: { content: memory(action.content) },
          },
        });
      case "revise_memory":
        return create(LongitudinalConsolidationActionSchema, {
          action: {
            case: "reviseMemory",
            value: {
              target: target(action.targetKey, "memory_revision"),
              intent: action.intent,
              content: memory(action.content),
            },
          },
        });
      case "create_schema":
        return create(LongitudinalConsolidationActionSchema, {
          action: {
            case: "createSchema",
            value: { content: schema(action.content) },
          },
        });
      case "revise_schema":
        return create(LongitudinalConsolidationActionSchema, {
          action: {
            case: "reviseSchema",
            value: {
              target: target(action.targetKey, "cognitive_schema_revision"),
              intent: action.intent,
              content: schema(action.content),
            },
          },
        });
      case "link_relation":
        return create(LongitudinalConsolidationActionSchema, {
          action: {
            case: "linkRelation",
            value: {
              from: endpoint(action.from, index),
              to: endpoint(action.to, index),
              relation: action.relation,
            },
          },
        });
    }
  });
  return create(CommitLongitudinalConsolidationRequestSchema, {
    operationId,
    subjectId: plan.subjectId,
    expectedAuthoritySeq: plan.authoritySeq,
    source: plan.consolidationSource,
    context: plan.candidates.map((candidate) => candidate.target!),
    producer,
    actions,
  });
}
