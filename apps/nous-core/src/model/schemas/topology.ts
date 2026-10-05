import { z } from "zod";
const key = z.string().min(1).max(36);
const reason = z.string().min(1).max(1024);
const supports = z.array(key).min(1).max(16);
const content = z.strictObject({
  label: z.string().min(1).max(256),
  description: z.string().max(4096).nullable(),
  kind_hint: z.string().max(128).nullable(),
});
const newTag = z.strictObject({
  key: z.string().regex(/^new_[a-z0-9_]{1,32}$/),
  content,
});
export const topologyMaintenanceSchema = z.strictObject({
  actions: z
    .array(
      z.discriminatedUnion("action", [
        z.strictObject({ action: z.literal("reuse_tag"), tagKey: key }),
        z.strictObject({
          action: z.literal("create_tag"),
          cognitionKeys: z.array(key).min(1).max(16),
          key: newTag.shape.key,
          content,
          supportKeys: supports,
          reason,
        }),
        z.strictObject({
          action: z.literal("revise_tag"),
          tagKey: key,
          content,
          supportKeys: supports,
          reason,
        }),
        z.strictObject({
          action: z.literal("attach_tag"),
          cognitionKey: key,
          tagKey: key,
          supportKeys: supports,
          reason,
        }),
        z.strictObject({
          action: z.literal("detach_tag"),
          associationKey: key,
          supportKeys: supports,
          reason,
        }),
        z.strictObject({
          action: z.literal("create_association"),
          fromKey: key,
          toKey: key,
          relation: z.enum([
            "tag_attachment",
            "assoc.related",
            "assoc.co_occurs",
            "assoc.sequence",
            "assoc.procedural",
            "assoc.shared_outcome",
          ]),
          supportKeys: supports,
          reason,
        }),
        z.strictObject({
          action: z.literal("revoke_association"),
          associationKey: key,
          supportKeys: supports,
          reason,
        }),
        z.strictObject({
          action: z.literal("merge_tags"),
          survivorKey: key,
          retiredKeys: z.array(key).min(1).max(7),
          supportKeys: supports,
          reason,
        }),
        z.strictObject({
          action: z.literal("split_tag"),
          tagKey: key,
          children: z.array(newTag).min(2).max(8),
          supportKeys: supports,
          reason,
        }),
        z.strictObject({ action: z.literal("no_change") }),
      ]),
    )
    .min(1)
    .max(16),
});
