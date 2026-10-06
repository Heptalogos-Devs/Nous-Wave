import { expect, it } from "vitest";
import { ConnectError, Code } from "@connectrpc/connect";
import { ResolveIdentityResponseSchema } from "@nous-wave/protocol/nous/wave/v1alpha1/identity_pb.js";
import { NousError } from "./index.js";
it("preserves identity ambiguity candidates at the official Client boundary", () => {
  const result = {
    status: "AMBIGUOUS_REFERENCE",
    candidates: [
      { canonical: { kind: "entity", value: "Alice-1" } },
      { canonical: { kind: "entity", value: "Alice-2" } },
    ],
  };
  const error = new NousError(
    new ConnectError(result.status, Code.InvalidArgument, undefined, [
      { desc: ResolveIdentityResponseSchema, value: result },
    ]),
  );
  expect(error).toMatchObject({
    message: result.status,
    candidates: result.candidates,
  });
  expect(error.details).toMatchObject([{ status: result.status }]);
});
