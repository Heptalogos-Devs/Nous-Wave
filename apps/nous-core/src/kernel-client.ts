import {
  coreExecutionSchema,
  type CoreExecutionPolicy,
} from "./configuration-catalog.js";
import { createClient, type Transport } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  AuthorityService,
  ArtifactStreamService,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/kernel_pb.js";
import { Health } from "@nous-wave/protocol/grpc/health/v1/health_pb.js";
import { ModelMaterialService } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/model_pb.js";

import { ConfigurationService } from "@nous-wave/protocol/nous/wave/v1alpha1/configuration_pb.js";

import { KernelConfigurationService } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/configuration_pb.js";
export class KernelClient {
  readonly hostRuntime;
  readonly configuration;
  readonly authority;
  readonly artifacts;
  readonly health;
  readonly modelMaterial;

  constructor(
    transport: Transport,
    readonly execution: CoreExecutionPolicy = coreExecutionSchema.parse(
      undefined,
    ),
  ) {
    this.hostRuntime = createClient(KernelConfigurationService, transport);
    this.configuration = createClient(ConfigurationService, transport);
    this.authority = createClient(AuthorityService, transport);
    this.artifacts = createClient(ArtifactStreamService, transport);
    this.health = createClient(Health, transport);
    this.modelMaterial = createClient(ModelMaterialService, transport);
  }

  static connect(
    endpoint: string,
    token: string,
    execution = coreExecutionSchema.parse(undefined),
  ) {
    return new KernelClient(
      createGrpcTransport({
        baseUrl: endpoint,
        defaultTimeoutMs: execution.kernel_rpc_timeout_ms,
        readMaxBytes: 8 * 1024 * 1024 + 65536,
        writeMaxBytes: 8 * 1024 * 1024 + 65536,
        interceptors: [
          (next) => async (request) => {
            request.header.set("authorization", "Bearer " + token);
            return next(request);
          },
        ],
      }),
      execution,
    );
  }
}
