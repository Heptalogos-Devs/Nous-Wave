export type ResourceBinding = {
  resourceRef: string;
  adapterKind: string;
  providerProfile: string;
  providerLocator: string;
};
export type StableExternalRef = {
  providerKind: string;
  providerProfile: string;
  profileDigest: string;
  resourceRef: string;
  providerResourceId: string;
  entryId: string;
  entryVersion: string | null;
  contentDigest: string;
  sourceLocator: string;
  retrievedAt: string;
  accessScope: string;
};
export type ResourceRecord = {
  resourceRef: string;
  reference: StableExternalRef;
  title?: string;
  content: string;
  providerRank: number;
  providerScore?: number;
  versionStatus: "current" | "stale" | "missing" | "unknown";
  accessStatus: "allowed" | "denied" | "unknown";
};
type VersionCheck = {
  status: ResourceRecord["versionStatus"];
  checkedAt: string;
};
type AccessCheck = {
  status: ResourceRecord["accessStatus"];
  checkedAt: string;
};
type ResourceMaterial = {
  reference: StableExternalRef;
  content: string;
  mediaType: string;
  version: VersionCheck;
  access: AccessCheck;
};
export interface ExternalResourceAdapter {
  readonly profileDigest: string;
  describe(
    binding: ResourceBinding,
    signal?: AbortSignal,
  ): Promise<{ resourceIds: string[]; accessible: boolean }>;
  search(
    binding: ResourceBinding,
    question: string,
    limit: number,
    signal?: AbortSignal,
  ): Promise<ResourceRecord[]>;
  materialize(
    reference: StableExternalRef,
    signal?: AbortSignal,
  ): Promise<ResourceMaterial>;
  checkVersion(
    reference: StableExternalRef,
    signal?: AbortSignal,
  ): Promise<VersionCheck>;
  checkAccess(
    reference: StableExternalRef,
    signal?: AbortSignal,
  ): Promise<AccessCheck>;
}
export class ResourceProviderError extends Error {
  constructor(
    readonly status: "unavailable" | "denied" | "stale" | "failed",
    message: string,
    readonly missing = false,
  ) {
    super(message);
  }
}
