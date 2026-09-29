import { createClient, type Transport } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  AuthorityService,
  ArtifactStreamService,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/kernel_pb.js";
import { Health } from "@nous-wave/protocol/grpc/health/v1/health_pb.js";
import { ModelMaterialService } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/model_pb.js";

export class KernelClient {
  readonly authority;
  readonly artifacts;
  readonly health;
  readonly modelMaterial;

  constructor(transport: Transport) {
    this.authority = createClient(AuthorityService, transport);
    this.artifacts = createClient(ArtifactStreamService, transport);
    this.health = createClient(Health, transport);
    this.modelMaterial = createClient(ModelMaterialService, transport);
  }

  static connect(endpoint: string, token: string) {
    return new KernelClient(
      createGrpcTransport({
        baseUrl: endpoint,
        defaultTimeoutMs: 30_000,
        interceptors: [
          (next) => async (request) => {
            request.header.set("authorization", "Bearer " + token);
            return next(request);
          },
        ],
      }),
    );
  }
}
