// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { z } from "zod";
import { createHash } from "node:crypto";
import { canonicalDigest } from "../../digest.js";
import type { ResourceProfiles } from "../configuration.js";
import {
  ResourceProviderError,
  type ExternalResourceAdapter,
  type ResourceBinding,
  type StableExternalRef,
  type ResourceRecord,
} from "../adapter.js";

const id = z.string().regex(/^[A-Za-z0-9_-]{1,128}$/);
const selectorSchema = z.strictObject({
  dataset_ids: z.array(id).min(1).max(16),
  document_ids: z.array(id).max(64).optional(),
});
const hitSchema = z.object({
  id,
  dataset_id: id,
  document_id: id,
  content: z.string(),
  document_keyword: z.string().max(1024).optional(),
  similarity: z.number().finite().optional(),
});
const chunkSchema = z.object({
  id,
  doc_id: id,
  content_with_weight: z.string(),
  available_int: z.union([z.literal(0), z.literal(1)]),
});
const hash = (content: string) =>
  createHash("sha256").update(content).digest("hex");

export class RagflowAdapter implements ExternalResourceAdapter {
  readonly profileDigest: string;
  constructor(
    private readonly name: string,
    private readonly profile: ResourceProfiles[string],
    private readonly credential: string,
  ) {
    this.profileDigest = canonicalDigest({
      name,
      profile,
      implementation: "ragflow-http-chunks-v1",
    });
  }
  private selector(binding: ResourceBinding) {
    if (
      binding.adapterKind !== "ragflow" ||
      binding.providerProfile !== this.name ||
      !binding.resourceRef ||
      binding.providerLocator.length > 4096
    )
      throw new ResourceProviderError("failed", "Resource binding is invalid");
    return selectorSchema.parse(JSON.parse(binding.providerLocator));
  }
  private async call(
    path: string,
    signal?: AbortSignal,
    body?: unknown,
  ): Promise<unknown> {
    let response: Response;
    try {
      if (signal?.aborted) throw signal.reason;
      response = await fetch(`${this.profile.base_url}${path}`, {
        method: body === undefined ? "GET" : "POST",
        redirect: "error",
        signal: signal
          ? AbortSignal.any([
              signal,
              AbortSignal.timeout(this.profile.timeout_ms),
            ])
          : AbortSignal.timeout(this.profile.timeout_ms),
        headers: {
          Authorization: `Bearer ${this.credential}`,
          "Content-Type": "application/json",
        },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    } catch {
      if (signal?.aborted) throw signal.reason;
      throw new ResourceProviderError(
        "unavailable",
        "Resource provider transport unavailable",
      );
    }
    if (!response.ok || !response.body) {
      await response.body?.cancel();
      throw new ResourceProviderError(
        response.status === 401 || response.status === 403
          ? "denied"
          : response.status === 404
            ? "stale"
            : "failed",
        `Resource provider HTTP ${response.status}`,
        response.status === 404,
      );
    }
    const chunks: Uint8Array[] = [];
    let size = 0;
    const reader = response.body.getReader();
    try {
      while (true) {
        const chunk = await reader.read();
        if (chunk.done) break;
        size += chunk.value.byteLength;
        if (size > 8 * 1024 * 1024) {
          await reader.cancel();
          throw new ResourceProviderError(
            "failed",
            "Resource provider response exceeds 8 MiB",
          );
        }
        chunks.push(chunk.value);
      }
      const envelope = z
        .object({ code: z.number().int(), data: z.unknown().optional() })
        .parse(JSON.parse(Buffer.concat(chunks).toString("utf8")));
      if (envelope.code !== 0)
        throw new ResourceProviderError(
          envelope.code === 100 ? "stale" : "failed",
          "Resource provider rejected operation",
          envelope.code === 100,
        );
      return envelope.data;
    } catch (error) {
      if (signal?.aborted) throw signal.reason;
      if (error instanceof ResourceProviderError) throw error;
      throw new ResourceProviderError(
        "failed",
        "Resource provider response failed validation",
      );
    } finally {
      reader.releaseLock();
    }
  }
  private bounded(content: string) {
    if (Buffer.byteLength(content) > this.profile.max_material_bytes)
      throw new ResourceProviderError(
        "failed",
        "Resource material exceeds configured byte bound",
      );
    return content;
  }
  async describe(binding: ResourceBinding, signal?: AbortSignal) {
    const { dataset_ids } = this.selector(binding);
    const accessible: string[] = [];
    for (const dataset of dataset_ids) {
      const result = z
        .array(z.object({ id }))
        .parse(
          await this.call(
            `/datasets?id=${encodeURIComponent(dataset)}&page_size=1`,
            signal,
            undefined,
          ),
        );
      if (result.some((item) => item.id === dataset)) accessible.push(dataset);
    }
    return {
      resourceIds: accessible,
      accessible: accessible.length === dataset_ids.length,
    };
  }
  async search(
    binding: ResourceBinding,
    question: string,
    limit: number,
    signal?: AbortSignal,
  ): Promise<ResourceRecord[]> {
    const selector = this.selector(binding);
    if (
      !question.trim() ||
      Buffer.byteLength(question) > 32768 ||
      !Number.isInteger(limit) ||
      limit < 1 ||
      limit > 64
    )
      throw new ResourceProviderError(
        "failed",
        "Resource search intent or limit is invalid",
      );
    const data = z.object({ chunks: z.array(hitSchema).max(64) }).parse(
      await this.call("/retrieval", signal, {
        question,
        ...selector,
        page: 1,
        page_size: limit,
        highlight: false,
      }),
    );
    if (data.chunks.length > limit)
      throw new ResourceProviderError(
        "failed",
        "Resource provider exceeded requested result limit",
      );
    const seen = new Set<string>();
    return data.chunks.map((hit, index) => {
      if (
        !selector.dataset_ids.includes(hit.dataset_id) ||
        (selector.document_ids?.length &&
          !selector.document_ids.includes(hit.document_id))
      )
        throw new ResourceProviderError(
          "failed",
          "Resource provider returned a record outside its selector",
        );
      const sourceLocator = JSON.stringify({
        dataset: hit.dataset_id,
        document: hit.document_id,
        chunk: hit.id,
      });
      if (seen.has(sourceLocator))
        throw new ResourceProviderError(
          "failed",
          "Resource provider returned duplicate record identity",
        );
      seen.add(sourceLocator);
      const content = this.bounded(hit.content);
      return {
        resourceRef: binding.resourceRef,
        reference: {
          providerKind: "ragflow",
          providerProfile: this.name,
          profileDigest: this.profileDigest,
          resourceRef: binding.resourceRef,
          providerResourceId: hit.dataset_id,
          entryId: hit.id,
          entryVersion: null,
          contentDigest: hash(content),
          sourceLocator,
          retrievedAt: new Date().toISOString(),
          accessScope: binding.providerLocator,
        },
        title: hit.document_keyword,
        content,
        providerRank: index + 1,
        providerScore: hit.similarity,
        versionStatus: "unknown",
        accessStatus: "unknown",
      };
    });
  }
  private async inspect(reference: StableExternalRef, signal?: AbortSignal) {
    if (
      reference.providerKind !== "ragflow" ||
      reference.providerProfile !== this.name ||
      reference.profileDigest !== this.profileDigest
    )
      throw new ResourceProviderError(
        "stale",
        "Resource provider profile has changed",
      );
    const locator = z
      .strictObject({ dataset: id, document: id, chunk: id })
      .parse(JSON.parse(reference.sourceLocator));
    const selector = selectorSchema.parse(JSON.parse(reference.accessScope));
    if (
      locator.dataset !== reference.providerResourceId ||
      locator.chunk !== reference.entryId ||
      !selector.dataset_ids.includes(locator.dataset) ||
      (selector.document_ids?.length &&
        !selector.document_ids.includes(locator.document))
    )
      throw new ResourceProviderError(
        "denied",
        "Resource record is outside its bound access selector",
      );
    const chunk = chunkSchema.parse(
      await this.call(
        `/datasets/${encodeURIComponent(locator.dataset)}/documents/${encodeURIComponent(locator.document)}/chunks/${encodeURIComponent(locator.chunk)}`,
        signal,
        undefined,
      ),
    );
    if (chunk.id !== locator.chunk || chunk.doc_id !== locator.document)
      throw new ResourceProviderError(
        "failed",
        "Resource provider returned a different entry",
      );
    if (!chunk.available_int)
      throw new ResourceProviderError(
        "denied",
        "Resource entry is unavailable for retrieval",
      );
    const content = this.bounded(chunk.content_with_weight);
    return {
      content,
      current: hash(content) === reference.contentDigest,
      checkedAt: new Date().toISOString(),
    };
  }
  async materialize(reference: StableExternalRef, signal?: AbortSignal) {
    const inspected = await this.inspect(reference, signal);
    if (!inspected.current)
      throw new ResourceProviderError(
        "stale",
        "Resource entry content changed",
      );
    return {
      reference,
      content: inspected.content,
      mediaType: "text/plain; charset=utf-8",
      version: { status: "current" as const, checkedAt: inspected.checkedAt },
      access: { status: "allowed" as const, checkedAt: inspected.checkedAt },
    };
  }
  async checkVersion(reference: StableExternalRef, signal?: AbortSignal) {
    try {
      const inspected = await this.inspect(reference, signal);
      return {
        status: inspected.current ? ("current" as const) : ("stale" as const),
        checkedAt: inspected.checkedAt,
      };
    } catch (error) {
      if (signal?.aborted) throw error;
      if (error instanceof ResourceProviderError && error.status === "stale")
        return {
          status: error.missing ? ("missing" as const) : ("stale" as const),
          checkedAt: new Date().toISOString(),
        };
      throw error;
    }
  }
  async checkAccess(reference: StableExternalRef, signal?: AbortSignal) {
    try {
      const inspected = await this.inspect(reference, signal);
      return { status: "allowed" as const, checkedAt: inspected.checkedAt };
    } catch (error) {
      if (signal?.aborted) throw error;
      if (error instanceof ResourceProviderError && error.status === "denied")
        return {
          status: "denied" as const,
          checkedAt: new Date().toISOString(),
        };
      throw error;
    }
  }
}
