// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import {
  ConfigExposure,
  ConfigurationView,
  configurationValue,
} from "@nous-wave/client";
import { connectNousInstance } from "@nous-wave/client/node";
import { webSource } from "@nous-wave/client";
import { readFile, mkdir, rename, writeFile } from "node:fs/promises";
import { dirname, extname, join, resolve } from "node:path";
import { parseArgs } from "node:util";

type Selection = {
  subjectId?: string;
  sessionId?: string;
  workContextId?: string;
};
import {
  CliError,
  commandInventory,
  uniqueReference,
  boundedInteger,
} from "./agent.js";

export async function runCli(args: string[], connect = connectNousInstance) {
  const { values, positionals } = parseArgs({
    args,
    allowPositionals: true,
    options: {
      "run-root": { type: "string" },
      "instance-root": { type: "string" },
      json: { type: "boolean", default: false },
      text: { type: "string" },
      source: { type: "string" },
      "media-type": { type: "string" },
      strategy: { type: "string" },
      target: { type: "string" },
      supersedes: { type: "string" },
      representation: { type: "string" },
      "operation-id": { type: "string" },
      "aboutness-mode": { type: "string" },
      aboutness: { type: "string", multiple: true },
      tag: { type: "string", multiple: true },
      "query-file": { type: "string" },
      subject: { type: "string" },
      session: { type: "string" },
      "work-context": { type: "string" },
      kind: { type: "string" },
      name: { type: "string" },
      canonical: { type: "string" },
      alias: { type: "string", multiple: true },
      "lexical-ref": { type: "string" },
      "association-file": { type: "string" },
      "request-file": { type: "string" },
      description: { type: "string" },
      "kind-hint": { type: "string" },
      "event-id": { type: "string" },
      "occurred-at": { type: "string" },
      consumer: { type: "string", default: "consumer:nous-cli:default" },
      "max-operations": { type: "string", default: "1" },
      "max-model-calls": { type: "string", default: "1" },
      "max-elapsed-ms": { type: "string", default: "30000" },
      "page-token": { type: "string" },
      "page-size": { type: "string", default: "50" },
      "max-nodes": { type: "string", default: "64" },
      "max-depth": { type: "string", default: "2" },
      desired: { type: "boolean" },
      advanced: { type: "boolean" },
      developer: { type: "boolean" },
      "max-batches": { type: "string", default: "16" },
    },
  });
  const stateFile = values["instance-root"]
    ? join(resolve(values["instance-root"]), "consumers/nous-cli.json")
    : undefined;
  async function selection(): Promise<Selection> {
    if (!stateFile) return {};
    try {
      const value: unknown = JSON.parse(await readFile(stateFile, "utf8"));
      if (!value || typeof value !== "object")
        throw new Error("Invalid CLI selection state");
      const state = value as Record<string, unknown>;
      for (const key of ["subjectId", "sessionId", "workContextId"])
        if (state[key] !== undefined && typeof state[key] !== "string")
          throw new Error("Invalid CLI selection state");
      return {
        subjectId: state.subjectId as string | undefined,
        sessionId: state.sessionId as string | undefined,
        workContextId: state.workContextId as string | undefined,
      };
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return {};
      throw error;
    }
  }
  async function save(state: Selection) {
    if (!stateFile)
      throw new Error("Use the nous launcher or provide --instance-root");
    await mkdir(dirname(stateFile), { recursive: true });
    const tmp = `${stateFile}.${crypto.randomUUID()}.tmp`;
    await writeFile(tmp, JSON.stringify(state), { mode: 0o600 });
    await rename(tmp, stateFile);
  }
  function required(value: string | undefined, name: string): string {
    if (!value) throw new Error(`${name} required`);
    return value;
  }
  function mediaType(path: string) {
    const types: Record<string, string> = {
      ".txt": "text/plain",
      ".md": "text/markdown",
      ".json": "application/json",
      ".png": "image/png",
      ".jpg": "image/jpeg",
      ".jpeg": "image/jpeg",
      ".webp": "image/webp",
      ".wav": "audio/wav",
      ".mp3": "audio/mpeg",
      ".ogg": "audio/ogg",
      ".mp4": "video/mp4",
      ".webm": "video/webm",
    };
    return (
      values["media-type"] ??
      types[extname(path).toLowerCase()] ??
      "application/octet-stream"
    );
  }
  async function execute() {
    const [command, action, argument] = positionals;
    if (!command || command === "help") return commandInventory;
    if (!values["run-root"])
      throw new CliError(
        "INVALID_ARGUMENT",
        "Provide --run-root or use the nous launcher",
      );
    const client = await connect({
      runRoot: resolve(values["run-root"]),
    });
    const selected = await selection();
    const state = {
      subjectId: values.subject ?? selected.subjectId,
      sessionId: values.session ?? selected.sessionId,
      workContextId: values["work-context"] ?? selected.workContextId,
    };
    if (command === "config") {
      const exposureCeiling = values.developer
        ? ConfigExposure.DEVELOPER
        : values.advanced
          ? ConfigExposure.ADVANCED
          : ConfigExposure.STANDARD;
      if (action === "list")
        return client.configuration.list({ exposureCeiling });
      const path = required(argument, "Configuration path");
      if (action === "describe") return client.configuration.describe(path);
      if (action === "get")
        return client.configuration.get({
          paths: [path],
          subjectId: values.subject,
          view: values.desired
            ? ConfigurationView.DESIRED
            : ConfigurationView.ACTIVE,
        });
      const operationId = values["operation-id"] ?? crypto.randomUUID();
      if (action === "set") {
        const value = configurationValue(
          required(positionals[3], "JSON value"),
        );
        return values.subject
          ? client.configuration.setSubject({
              operationId,
              path,
              value,
              subjectId: values.subject,
            })
          : client.configuration.setSystem({ operationId, path, value });
      }
      if (action === "clear")
        return values.subject
          ? client.configuration.clearSubject({
              operationId,
              path,
              subjectId: values.subject,
            })
          : client.configuration.clearSystem({ operationId, path });
      throw new Error("Use config list|describe|get|set|clear");
    }
    if (command === "status")
      return {
        status: await client.system.status({}),
        capabilities: await client.system.capabilities({}),
      };
    if (command === "subject") {
      if (action === "list") return client.subjects.list({});
      if (action === "create") {
        const subject = await client.subjects.create({
          operationId: crypto.randomUUID(),
          capabilities: { memory: true },
          cognitiveSeed: {
            text: "schema_version = 1",
            format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
          },
        });
        await save({ subjectId: subject.subjectId });
        return subject;
      }
      if (action === "use") {
        const subject = await client.subjects.get({
          subjectId: required(argument, "Subject ID"),
        });
        await save({ subjectId: subject.subjectId });
        return subject;
      }
    }
    const subjectId = required(
      state.subjectId,
      "Selected Subject (run subject create/use)",
    );
    if (command === "session") {
      if (action === "open") {
        const session = await client.cognition.openSession({ subjectId });
        await save({ ...state, sessionId: session.sessionId });
        return session;
      }
      const id = required(
        state.sessionId,
        "Selected Session (run session open)",
      );
      if (action === "show")
        return client.cognition.getSession({ subjectId, id });
      if (action === "close") {
        const session = await client.cognition.closeSession({ subjectId, id });
        await save({ ...state, sessionId: undefined });
        return session;
      }
    }
    if (command === "observe") {
      const input = {
        subjectId,
        sessionId: state.sessionId,
        requestId: crypto.randomUUID(),
        ...(values.source ? webSource(values.source) : { sourceClass: "file" }),
        observedAt: {
          seconds: BigInt(Math.floor(Date.now() / 1000)),
          nanos: 0,
        },
        admit: true,
      };
      if (action === "text")
        return client.cognition.observe({
          ...input,
          material: {
            case: "inlineText",
            value: {
              text: required(values.text, "--text"),
              mediaType: "text/plain",
            },
          },
        });
      if (action === "file") {
        const path = required(argument, "File path");
        const artifact = await client.artifacts.uploadFile(subjectId, path, {
          mediaType: mediaType(path),
        });
        const observation = await client.cognition.observe({
          ...input,
          material: { case: "artifactId", value: artifact.artifactId },
        });
        return { artifact, ...observation };
      }
    }
    if (command === "derive")
      return client.model.deriveMaterial({
        subjectId,
        sourceRegionId: required(action, "Source region ID"),
        strategy: values.strategy,
        target: values.target,
        supersedes: values.supersedes,
      });
    if (command === "form") {
      const operationId = values["operation-id"] ?? crypto.randomUUID();
      const result = await client.model.formFromObservation({
        subjectId,
        operationId,
        aboutnessMode: values["aboutness-mode"],
        explicitAboutness: values.aboutness,
        explicitTags: await Promise.all(
          (values.tag ?? []).map(
            async (text) => (await resolveReference(text, "tag")).value,
          ),
        ),
        occurrenceId: required(action, "Occurrence ID"),
        representationId: values.representation,
      });
      return { ...result, operationId };
    }
    if (command === "embeddings" && action === "prepare") {
      const budget = Number(values["max-batches"]);
      if (!Number.isInteger(budget) || budget < 1 || budget > 10_000)
        throw new Error("--max-batches must be 1..10000");
      let committed = 0;
      for (let requests = 0; requests < budget; requests++) {
        const batch = await client.model.prepareEmbeddings({
          subjectId,
          limit: 64,
        });
        committed += batch.committed;
        if (batch.committed === 0 || batch.degradation.length)
          return {
            committed,
            requests: requests + 1,
            degradation: batch.degradation,
          };
      }
      return {
        committed,
        degradation: [
          {
            code: "caller_budget_exhausted",
            detail: "Embedding batch budget reached",
          },
        ],
      };
    }
    async function resolveReference(text: string, kind = "") {
      if (
        kind &&
        ![
          "entity",
          "resource",
          "memory",
          "memory_revision",
          "tag",
          "cognitive_schema",
          "cognitive_schema_revision",
          "episode",
          "episode_revision",
          "journal",
          "journal_revision",
          "occurrence",
          "external_object",
        ].includes(kind)
      )
        kind = "";
      if (
        kind === "tag" &&
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(
          text,
        )
      )
        return { kind, value: text.toLowerCase() };
      const canonical = /^([a-z_]+):(.+)$/.exec(text);
      if (
        canonical &&
        canonical[1] === kind &&
        (["entity", "resource"].includes(kind) ||
          /^[0-9a-f-]{36}$/i.test(canonical[2]!)) &&
        [
          "entity",
          "memory",
          "memory_revision",
          "episode_revision",
          "journal_revision",
          "tag",
          "cognitive_schema",
          "cognitive_schema_revision",
          "occurrence",
          "resource",
        ].includes(canonical[1]!)
      )
        return { kind, value: canonical[2]! };
      const result = await client.identity.resolve({
        subjectId,
        kind,
        locator: { case: "lexicalRef", value: text },
      });
      return uniqueReference(result);
    }
    if (command === "identity") {
      if (action === "resolve") {
        const result = await client.identity.resolve({
          subjectId,
          kind: values.kind ?? "",
          locator: values["lexical-ref"]
            ? { case: "lexicalRef", value: values["lexical-ref"] }
            : {
                case: "name",
                value: required(values.name, "--name or --lexical-ref"),
              },
        });
        uniqueReference(result);
        return result;
      }
      if (action === "bind")
        return client.identity.bind({
          subjectId,
          canonical: {
            kind: required(values.kind, "--kind"),
            value: required(values.canonical, "--canonical"),
          },
          displayName: required(values.name, "--name"),
          aliases: values.alias ?? [],
        });
    }
    async function requestPayload(path: string | undefined) {
      const text = await readFile(
        required(path, "--request-file or --association-file"),
        "utf8",
      );
      if (Buffer.byteLength(text) > 65536)
        throw new CliError(
          "INVALID_ARGUMENT",
          "Mutation request exceeds 64 KiB",
        );
      const parsed: unknown = JSON.parse(text);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed))
        throw new CliError(
          "INVALID_ARGUMENT",
          "Mutation request must be an object",
        );
      return parsed;
    }
    if (command === "maintenance" && action === "grant") {
      return client.cognition.grantMaintenance({
        subjectId,
        maxOperations: boundedInteger(
          values["max-operations"],
          1,
          32,
          "--max-operations",
        ),
        maxModelCalls: boundedInteger(
          values["max-model-calls"],
          0,
          32,
          "--max-model-calls",
        ),
        maxElapsedMs: boundedInteger(
          values["max-elapsed-ms"],
          1,
          300000,
          "--max-elapsed-ms",
        ),
      });
    }
    if (command === "tag") {
      const operationId = values["operation-id"] ?? crypto.randomUUID();
      if (action === "create")
        return client.concepts.createTag({
          subjectId,
          operationId,
          tag: {
            label: required(values.name, "--name"),
            description: values.description,
            kindHint: values["kind-hint"],
            origin: "host_explicit",
          },
        });
      if (action === "revise") {
        const payload = (await requestPayload(
          values["request-file"],
        )) as Parameters<typeof client.concepts.reviseTag>[0];
        return client.concepts.reviseTag({
          ...payload,
          subjectId,
          operationId,
        });
      }
      if (action === "merge") {
        const payload = (await requestPayload(
          values["request-file"],
        )) as Parameters<typeof client.concepts.mergeTags>[0];
        return client.concepts.mergeTags({
          ...payload,
          subjectId,
          operationId,
        });
      }
      if (action === "split") {
        const payload = (await requestPayload(
          values["request-file"],
        )) as Parameters<typeof client.concepts.splitTag>[0];
        return client.concepts.splitTag({ ...payload, subjectId, operationId });
      }
      if (action === "attach") {
        const payload = (await requestPayload(
          values["association-file"],
        )) as NonNullable<
          Parameters<typeof client.concepts.associate>[0]["association"]
        >;
        if (!Array.isArray(payload.supports) || !payload.supports.length)
          throw new CliError(
            "INVALID_ARGUMENT",
            "Attachment requires nonempty supports",
          );
        const tag = await resolveReference(
          required(values.tag?.[0], "--tag"),
          "tag",
        );
        const cognition = await resolveReference(
          required(argument, "Exact cognition revision"),
          argument?.split(":")[0] ?? "",
        );
        if (
          ![
            "memory_revision",
            "cognitive_schema_revision",
            "episode_revision",
            "journal_revision",
          ].includes(cognition.kind)
        )
          throw new CliError(
            "INVALID_ARGUMENT",
            "Attachment requires an exact cognition revision",
          );
        return client.concepts.associate({
          subjectId,
          operationId,
          association: {
            ...payload,
            from: {
              $typeName: "nous.wave.v1alpha1.CognitiveRef",
              ...cognition,
            },
            to: { $typeName: "nous.wave.v1alpha1.CognitiveRef", ...tag },
            relationKind: "tag_attachment",
            polarity: "positive",
            supportClass: "host_explicit",
          },
        });
      }
      if (action === "list" || action === "search") {
        const page = {
          pageToken: values["page-token"],
          pageSize: boundedInteger(values["page-size"], 1, 200, "--page-size"),
        };
        return action === "list"
          ? client.concepts.listTags({ subjectId, page })
          : client.concepts.searchTags({
              subjectId,
              text: required(argument, "Search text"),
              page,
            });
      }
      if (action === "resolve") {
        const result = await client.identity.resolve({
          subjectId,
          kind: "tag",
          locator: { case: "name", value: required(argument, "Tag name") },
        });
        uniqueReference(result);
        return result;
      }
      if (action === "get") {
        const ref = await resolveReference(
          required(argument, "Tag reference"),
          "tag",
        );
        return client.concepts.getTag({ subjectId, id: ref.value });
      }
    }
    if (command === "association") {
      if (action === "revoke")
        return client.concepts.revokeAssociation({
          subjectId,
          operationId: values["operation-id"] ?? crypto.randomUUID(),
          associationId: required(argument, "Association ID"),
        });
      if (action === "neighborhood") {
        const text = required(argument, "Reference");
        const kind = values.kind ?? text.split(":")[0] ?? "";
        const root = await resolveReference(text, kind);
        return client.concepts.neighborhood({
          subjectId,
          root,
          maxNodes: boundedInteger(values["max-nodes"], 1, 256, "--max-nodes"),
          maxDepth: boundedInteger(values["max-depth"], 1, 4, "--max-depth"),
        });
      }
      if (action === "create") {
        type Association = NonNullable<
          Parameters<typeof client.concepts.associate>[0]["association"]
        >;
        const payload: unknown = JSON.parse(
          await readFile(
            required(values["association-file"], "--association-file"),
            "utf8",
          ),
        );
        if (
          !payload ||
          typeof payload !== "object" ||
          !("supports" in payload) ||
          !Array.isArray(payload.supports) ||
          !payload.supports.length
        )
          throw new CliError(
            "INVALID_ARGUMENT",
            "Association requires nonempty supports",
          );
        return client.concepts.associate({
          subjectId,
          operationId: required(values["operation-id"], "--operation-id"),
          association: payload as Association,
        });
      }
    }
    if (command === "query") {
      const preparing = action === "prepare" || action === "inspect";
      const nousql = values["query-file"]
        ? await readFile(values["query-file"], "utf8")
        : required(preparing ? argument : action, "NousQL");
      if (Buffer.byteLength(nousql) > 32768)
        throw new CliError("INVALID_ARGUMENT", "NousQL exceeds 32 KiB");
      const request = {
        subjectId,
        sessionId: state.sessionId,
        workContextId: state.workContextId,
        nousql,
      };
      if (!preparing) return client.cognition.query(request);
      const prepared = await client.cognition.prepareQuery(request);
      const boundQuery: unknown = JSON.parse(prepared.boundQuery);
      return { ...prepared, boundQuery };
    }
    if (command === "use") {
      const kind = values.kind ?? "referenced";
      if (
        ![
          "presented",
          "referenced",
          "acted_on",
          "result_supported",
          "result_refuted",
          "corrected",
          "pinned",
        ].includes(kind)
      )
        throw new CliError("INVALID_ARGUMENT", "Invalid use kind");
      const text = required(action, "Exact cognition revision");
      const reference = await resolveReference(text, text.split(":")[0] ?? "");
      if (
        ![
          "memory_revision",
          "cognitive_schema_revision",
          "episode_revision",
          "journal_revision",
        ].includes(reference.kind)
      )
        throw new CliError(
          "INVALID_ARGUMENT",
          "Use requires an exact cognition revision",
        );
      const occurred = values["occurred-at"]
        ? new Date(values["occurred-at"])
        : new Date();
      if (
        !Number.isFinite(occurred.getTime()) ||
        (values["occurred-at"] &&
          !/(Z|[+-]\d{2}:\d{2})$/.test(values["occurred-at"]))
      )
        throw new CliError(
          "INVALID_ARGUMENT",
          "--occurred-at requires an ISO timestamp with offset",
        );
      return client.cognition.reportUse({
        subjectId,
        sessionId: state.sessionId,
        consumerRef: values.consumer,
        events: [
          {
            eventId: values["event-id"] ?? crypto.randomUUID(),
            reference,
            kind,
            occurredAt: {
              seconds: BigInt(Math.floor(occurred.getTime() / 1000)),
              nanos: occurred.getUTCMilliseconds() * 1_000_000,
            },
          },
        ],
      });
    }
    if (command === "trace") {
      const refText = required(action, "Reference");
      let reference: { kind: string; value: string };
      const canonical = /^(memory|memory_revision):(.+)$/.exec(refText);
      if (canonical) reference = { kind: canonical[1]!, value: canonical[2]! };
      else {
        const resolved = await client.identity.resolve({
          subjectId,
          kind: "",
          locator: { case: "lexicalRef", value: refText },
        });
        if (
          resolved.candidates.length !== 1 ||
          !resolved.candidates[0]?.canonical
        )
          throw new Error("Reference is not uniquely bound");
        reference = resolved.candidates[0].canonical;
      }
      const memory =
        reference.kind === "memory"
          ? await client.memory.get({ subjectId, id: reference.value })
          : await client.memory.revision({ subjectId, id: reference.value });
      const sources = [];
      const visited = new Set<string>();
      const derivations: unknown[] = [];
      const traceMaterial = async (materialRef: {
        kind: string;
        value: string;
      }): Promise<void> => {
        const key = `${materialRef.kind}:${materialRef.value}`;
        if (visited.has(key)) return;
        if (visited.size >= 512)
          throw new Error("Trace material budget exceeded");
        visited.add(key);
        if (materialRef.kind === "derived_representation") {
          const representation = await client.material.representation({
            subjectId,
            id: materialRef.value,
          });
          derivations.push({ reference: materialRef, representation });
          for (const input of representation.inputs)
            if (input.reference) await traceMaterial(input.reference);
        } else if (materialRef.kind === "derived_region") {
          const region = await client.material.derivedRegion({
            subjectId,
            id: materialRef.value,
          });
          derivations.push({ reference: materialRef, region });
          await traceMaterial({
            kind: "derived_representation",
            value: region.representationId,
          });
        } else if (materialRef.kind === "source_region") {
          const region = await client.material.sourceRegion({
            subjectId,
            id: materialRef.value,
          });
          const artifact = await client.material.getArtifact({
            subjectId,
            id: region.artifactId,
          });
          derivations.push({ reference: materialRef, region, artifact });
        }
      };
      for (const support of memory.supports) {
        if (support.support.case !== "evidence") continue;
        const evidence = support.support.value;
        const occurrence = await client.material.occurrence({
          subjectId,
          id: evidence.occurrenceId,
        });
        const artifact = occurrence.artifactId
          ? await client.material.getArtifact({
              subjectId,
              id: occurrence.artifactId,
            })
          : undefined;
        const representation =
          evidence.locator.case === "derivedRepresentationId"
            ? await client.material.representation({
                subjectId,
                id: evidence.locator.value,
              })
            : undefined;
        if (representation)
          await traceMaterial({
            kind: "derived_representation",
            value: representation.representationId,
          });
        if (evidence.locator.case === "derivedRegionId")
          await traceMaterial({
            kind: "derived_region",
            value: evidence.locator.value,
          });
        if (evidence.locator.case === "sourceRegionId")
          await traceMaterial({
            kind: "source_region",
            value: evidence.locator.value,
          });
        sources.push({ evidence, occurrence, artifact, representation });
      }
      const producer = memory.producerSignatureId
        ? await client.material.producer({
            subjectId,
            id: memory.producerSignatureId,
          })
        : undefined;
      return { memory, producer, sources, derivations };
    }
    if (command === "context") {
      if (action === "create") {
        const result = await client.cognition.createWorkContext({
          subjectId,
          operationId: crypto.randomUUID(),
          purpose: required(values.text, "--text purpose"),
        });
        if (result.workContext)
          await save({
            ...state,
            workContextId: result.workContext.workContextId,
          });
        return result;
      }
      const id = argument ?? required(state.workContextId, "WorkContext ID");
      const result = await client.cognition.getWorkContext({
        subjectId,
        workContextId: id,
      });
      if (action === "show") return result;
      if (action === "end")
        return client.cognition.endWorkContext({
          subjectId,
          workContextId: id,
          expectedRevision: result.workContext?.revision,
          operationId: crypto.randomUUID(),
        });
      if (action === "foreground") {
        const sessionId = required(state.sessionId, "Session ID");
        const session = await client.cognition.getSession({
          subjectId,
          id: sessionId,
        });
        return client.cognition.setActiveWorkContext({
          subjectId,
          sessionId,
          expectedRuntimeRevision: session.runtimeRevision,
          workContextId: id,
          operationId: crypto.randomUUID(),
        });
      }
    }
    throw new Error("Unknown command; run nous help");
  }
  return execute();
}
