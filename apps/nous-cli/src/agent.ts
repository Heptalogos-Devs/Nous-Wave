// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { NousError } from "@nous-wave/client";

export class CliError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly details: unknown[] = [],
    readonly candidates: unknown[] = [],
  ) {
    super(message);
  }
}

export function uniqueReference(result: {
  status: string;
  candidates: { canonical?: { kind: string; value: string } }[];
}) {
  const canonical = result.candidates[0]?.canonical;
  if (result.status !== "BOUND" || result.candidates.length !== 1 || !canonical)
    throw new CliError(
      result.status,
      "Reference must resolve uniquely",
      [],
      result.candidates,
    );
  return canonical;
}

export function boundedInteger(
  value: string | undefined,
  min: number,
  max: number,
  name: string,
) {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < min || parsed > max)
    throw new CliError("INVALID_ARGUMENT", `${name} must be ${min}..${max}`);
  return parsed;
}

export function cliErrorPayload(error: unknown) {
  if (error instanceof CliError)
    return {
      code: error.code,
      message: error.message,
      details: error.details,
      candidates: error.candidates,
    };
  const message =
    error instanceof Error ? error.message : "CLI operation failed";
  let parsed: unknown;
  try {
    parsed = JSON.parse(message);
  } catch {
    /* Plain transport/local message. */
  }
  if (
    parsed &&
    typeof parsed === "object" &&
    "code" in parsed &&
    typeof parsed.code === "string"
  )
    return { message, details: [], candidates: [], ...parsed };
  const domainCode =
    /\b(?:UNKNOWN_REFERENCE|AMBIGUOUS_REFERENCE|REFERENCE_TYPE_MISMATCH|REFERENCE_TOMBSTONED|STALE_CONTEXT|UNAVAILABLE)\b/.exec(
      message,
    )?.[0];
  const transportCodes: Record<number, string> = {
    3: "INVALID_ARGUMENT",
    5: "NOT_FOUND",
    8: "RESOURCE_EXHAUSTED",
    9: "FAILED_PRECONDITION",
    10: "STALE_CONTEXT",
    14: "UNAVAILABLE",
  };
  return {
    code:
      domainCode ??
      (error instanceof NousError
        ? (transportCodes[error.code] ?? "RPC_ERROR")
        : "INVALID_ARGUMENT"),
    message,
    details: error instanceof NousError ? error.details : [],
    candidates: error instanceof NousError ? error.candidates : [],
  };
}

export const commandInventory = {
  contextFlags: {
    subject: "Subject ID overrides local selection",
    session: "Session ID overrides local selection",
    "work-context": "WorkContext ID overrides local selection",
  },
  output: {
    success: "stdout JSON; int64 as decimal strings",
    error: "stderr JSON code/message/details/candidates; nonzero exit",
  },
  commands: [
    { command: "status", description: "Instance status and capabilities" },
    {
      command: "identity resolve",
      parameters: ["--kind <kind> --name <name> | --lexical-ref <ref>"],
      example: "nous identity resolve --kind entity --name Alice --json",
    },
    {
      command: "identity bind",
      parameters: [
        "--kind <kind>",
        "--canonical <id>",
        "--name <name>",
        "--alias <alias> (repeatable)",
      ],
      example:
        "nous identity bind --kind entity --canonical entity:alice --name Alice --alias A --json",
    },
    {
      command: "tag list|get|search|resolve|create|revise|merge|split|attach",
      parameters: [
        "get <tag-ref>",
        "search <text>",
        "resolve <name>",
        "create --name <label> [--description <text>]",
        "revise|merge|split --request-file <JSON>",
        "attach <exact-revision> --tag <ref> --association-file <supports JSON>",
      ],
      example: "nous tag resolve deploy --json",
    },
    {
      command: "association neighborhood",
      parameters: [
        "<canonical-or-lexical-ref>",
        "--kind <kind> (for lexical)",
        "--max-nodes 1..256",
        "--max-depth 1..4",
      ],
      example: "nous association neighborhood memory:<id> --json",
    },
    {
      command: "association create",
      parameters: ["--operation-id <uuid>", "--association-file <json>"],
      description:
        "Official Client Association object; requires nonempty revision/use-event supports",
    },
    {
      command: "association revoke",
      parameters: ["<association-id>", "--operation-id <uuid>"],
      description:
        "Revoke one exact AssociationEvidence, including a Tag attachment",
    },
    {
      command: "maintenance grant",
      parameters: [
        "--max-operations 1..32",
        "--max-model-calls 0..32",
        "--max-elapsed-ms 1..300000",
      ],
      description: "Host authorizes a bounded maintenance opportunity",
    },
    {
      command: "use",
      parameters: [
        "<exact cognition revision>",
        "--kind presented|referenced|acted_on|result_supported|result_refuted|corrected|pinned",
        "--event-id <uuid>",
        "--consumer <ref>",
        "--occurred-at <ISO timestamp>",
      ],
      description:
        "Typed meaningful use; stable retry reuses event ID and occurrence timestamp",
    },
    {
      command: "query",
      parameters: ["<NousQL> | --query-file <path>"],
      example:
        "nous query '\"deployment decision\" $return(memory) $limit(5)' --json",
    },
    {
      command: "query prepare|inspect",
      parameters: ["<NousQL> | --query-file <path>"],
      description:
        "Closed binding and representation inspection; no retrieval or provider call",
      example:
        "nous query prepare '\"deployment decision\" $return(memory)' --subject <id> --session <id> --work-context <id> --json",
    },
    {
      command: "config list|describe|get|set|clear",
      parameters: [
        "[path] [JSON value]",
        "--subject <id>",
        "--desired",
        "--advanced|--developer",
      ],
    },
    { command: "subject create|list|use", parameters: ["[id]"] },
    { command: "session open|show|close" },
    {
      command: "observe text|file",
      parameters: [
        "--text <text> --source <url> | <path>",
        "--media-type <mime>",
      ],
    },
    {
      command: "form",
      parameters: [
        "<occurrence-id>",
        "--operation-id <uuid>",
        "--aboutness <ref> (repeatable)",
        "--tag <Tag ID/ref/LexicalRef> (repeatable)",
      ],
    },
    {
      command: "derive",
      parameters: ["<source-region-id>", "--strategy <strategy>"],
    },
    { command: "embeddings prepare", parameters: ["--max-batches <n>"] },
    { command: "trace", parameters: ["<canonical-or-lexical-ref>"] },
    {
      command: "context create|foreground|show|end",
      parameters: ["[id]", "--text <purpose> (create)"],
    },
  ],
};
