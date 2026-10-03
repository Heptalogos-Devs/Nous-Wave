import { fromJson, type JsonValue } from "@bufbuild/protobuf";
import { ValueSchema } from "@bufbuild/protobuf/wkt";
export {
  ConfigExposure,
  ConfigurationView,
} from "@nous-wave/protocol/nous/wave/v1alpha1/configuration_pb.js";
import { ConfigurationService } from "@nous-wave/protocol/nous/wave/v1alpha1/configuration_pb.js";
import {
  createClient,
  ConnectError,
  type CallOptions,
  type Transport,
} from "@connectrpc/connect";
import {
  SubjectService,
  CognitionService,
  MemoryService,
  MaterialService,
} from "@nous-wave/protocol/nous/wave/v1alpha1/services_pb.js";
import { IdentityService } from "@nous-wave/protocol/nous/wave/v1alpha1/identity_pb.js";

/** A consumer-owned web identity, with the original public locator preserved. */
export function webSource(value: string) {
  const url = new URL(value);
  if (
    !["http:", "https:"].includes(url.protocol) ||
    url.username ||
    url.password
  )
    throw new Error("Source URL must be credential-free HTTP(S)");
  const source = url.toString();
  return {
    sourceClass: "web",
    externalObjectRef: `object:web:${source}`,
    context: { source_url: source },
  };
}
import {
  ResourceService,
  TopologyService,
  SystemService,
} from "@nous-wave/protocol/nous/wave/v1alpha1/management_pb.js";
import { ModelService } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";

// Transport metadata is removed at the official Client boundary.
type Data<T> = T extends Uint8Array
  ? Uint8Array
  : T extends readonly (infer I)[]
    ? Data<I>[]
    : T extends object
      ? { [K in keyof T as K extends `$${string}` ? never : K]: Data<T[K]> }
      : T;
export interface RequestOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
}
export class NousError extends Error {
  readonly code: number;
  readonly details: readonly unknown[];
  constructor(error: ConnectError) {
    super(error.rawMessage, { cause: error });
    this.name = "NousError";
    this.code = error.code;
    this.details = error.details;
  }
}
function plain<T>(value: T): Data<T> {
  if (value instanceof Uint8Array) return value as Data<T>;
  if (Array.isArray(value)) return value.map(plain) as Data<T>;
  if (value !== null && typeof value === "object")
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !key.startsWith("$"))
        .map(([key, val]) => [key, plain(val)]),
    ) as Data<T>;
  return value as Data<T>;
}
function call<I, O>(method: (input: I, options?: CallOptions) => Promise<O>) {
  return async (input: I, options?: RequestOptions): Promise<Data<O>> => {
    try {
      return plain(await method(input, options));
    } catch (error) {
      if (error instanceof ConnectError) throw new NousError(error);
      throw error;
    }
  };
}
export function createNousClient(transport: Transport) {
  const subjects = createClient(SubjectService, transport);
  const cognition = createClient(CognitionService, transport);
  const memory = createClient(MemoryService, transport);
  const material = createClient(MaterialService, transport);
  const identity = createClient(IdentityService, transport);
  const resources = createClient(ResourceService, transport);
  const topology = createClient(TopologyService, transport);
  const system = createClient(SystemService, transport);
  const model = createClient(ModelService, transport);
  const configuration = createClient(ConfigurationService, transport);
  return {
    configuration: {
      list: call(configuration.listConfigDescriptors),
      describe: call(configuration.getConfigDescriptor),
      get: call(configuration.getConfiguration),
      setSystem: call(configuration.setSystemOverride),
      clearSystem: call(configuration.clearSystemOverride),
      setSubject: call(configuration.setSubjectOverride),
      clearSubject: call(configuration.clearSubjectOverride),
    },
    identity: {
      bind: call(identity.bindIdentity),
      resolve: call(identity.resolveIdentity),
    },
    model: {
      formFromObservation: call(model.formFromObservation),
      deriveMaterial: call(model.deriveMaterial),
      prepareEmbeddings: call(model.prepareEmbeddings),
    },
    resources: {
      materialize: call(resources.materializeResource),
      put: call(resources.putResource),
      get: call(resources.getResource),
      list: call(resources.listResources),
      remove: call(resources.removeResource),
    },
    topology: {
      createTag: call(topology.createTag),
      getTag: call(topology.getTag),
      listTags: call(topology.listTags),
      associate: call(topology.createAssociation),
      revokeAssociation: call(topology.revokeAssociation),
      neighborhood: call(topology.getNeighborhood),
      rebindEntity: call(topology.rebindEntity),
      createSchema: call(topology.createCognitiveSchema),
      getSchema: call(topology.getCognitiveSchema),
      addSchemaEvidence: call(topology.addSchemaEvidence),
      reviseSchema: call(topology.reviseCognitiveSchema),
      splitSchema: call(topology.splitCognitiveSchema),
      mergeSchemas: call(topology.mergeCognitiveSchemas),
    },
    system: {
      status: call(system.getStatus),
      capabilities: call(system.getCapabilities),
      projections: call(system.getProjectionStatus),
    },
    subjects: {
      create: call(subjects.createSubject),
      get: call(subjects.getSubject),
      list: call(subjects.listSubjects),
      seed: call(subjects.getCognitiveSeed),
      adoptSeed: call(subjects.adoptCognitiveSeed),
    },
    cognition: {
      openSession: call(cognition.openSession),
      getSession: call(cognition.getSession),
      listSessions: call(cognition.listSessions),
      closeSession: call(cognition.closeSession),
      observe: call(cognition.recordObservation),
      query: call(cognition.query),
      reportUse: call(cognition.reportUse),
      grantMaintenance: call(cognition.grantMaintenance),
      recall: async (
        subjectId: string,
        nousql: string,
        options?: RequestOptions,
      ) => {
        return call(cognition.query)({ subjectId, nousql }, options);
      },
      createWorkContext: call(cognition.createWorkContext),
      getWorkContext: call(cognition.getWorkContext),
      listWorkContexts: call(cognition.listWorkContexts),
      updateWorkContext: call(cognition.updateWorkContext),
      pauseWorkContext: call(cognition.pauseWorkContext),
      resumeWorkContext: call(cognition.resumeWorkContext),
      endWorkContext: call(cognition.endWorkContext),
      setActiveWorkContext: call(cognition.setActiveWorkContext),
      project: call(cognition.buildProjection),
      managedContext: call(cognition.buildManagedContext),
    },
    memory: {
      setAccessibility: call(memory.setAccessibility),
      linkRevisions: call(memory.linkRevisions),
      get: call(memory.getMemory),
      list: call(memory.listMemories),
      revision: call(memory.getMemoryRevision),
      history: call(memory.listMemoryRevisions),
      form: call(memory.formMemory),
      revise: call(memory.reviseMemory),
      suppress: call(memory.suppressMemory),
      restore: call(memory.restoreMemory),
      withdraw: call(memory.withdrawMemory),
      reaccept: call(memory.reacceptMemory),
      purge: call(memory.purgeMemory),
      consolidate: call(memory.consolidateMemory),
      createEpisode: call(memory.createEpisode),
      getEpisode: call(memory.getEpisode),
      getEpisodeRevision: call(memory.getEpisodeRevision),
      listEpisodes: call(memory.listEpisodes),
      listEpisodeRevisions: call(memory.listEpisodeRevisions),
      reviseEpisode: call(memory.reviseEpisode),
      linkEpisodeRevisions: call(memory.linkEpisodeRevisions),
      suppressEpisode: call(memory.suppressEpisode),
      restoreEpisode: call(memory.restoreEpisode),
      withdrawEpisode: call(memory.withdrawEpisode),
      reacceptEpisode: call(memory.reacceptEpisode),
      purgeEpisode: call(memory.purgeEpisode),
      getJournal: call(memory.getJournal),
      getJournalRevision: call(memory.getJournalRevision),
      listJournals: call(memory.listJournals),
      listJournalRevisions: call(memory.listJournalRevisions),
      suppressJournal: call(memory.suppressJournal),
      restoreJournal: call(memory.restoreJournal),
      withdrawJournal: call(memory.withdrawJournal),
      reacceptJournal: call(memory.reacceptJournal),
      purgeJournal: call(memory.purgeJournal),
    },
    material: {
      derivedRegion: call(material.getDerivedRegion),
      producer: call(material.getProducer),
      representations: call(material.listDerivedRepresentations),
      limits: call(material.getLimits),
      getArtifact: call(material.getArtifact),
      listArtifacts: call(material.listArtifacts),
      materialize: call(material.materializeEvidence),
      occurrence: call(material.getOccurrence),
      sourceRegion: call(material.getSourceRegion),
      representation: call(material.getDerivedRepresentation),
    },
  };
}
export type NousClient = ReturnType<typeof createNousClient>;

/** CLI and management callers use one JSON syntax for scalar and structured values. */
export function configurationValue(text: string) {
  return fromJson(ValueSchema, JSON.parse(text) as JsonValue);
}
