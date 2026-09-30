import { expect, it, vi } from "vitest";
import { dirname } from "node:path";
import { stat } from "node:fs/promises";
const observed = vi.hoisted(() => ({
  commands: [] as string[][],
  roots: [] as string[],
  tokenInherited: false,
  failFrames: false,
}));
vi.mock("node:child_process", async (original) => {
  const actual = await original<typeof import("node:child_process")>();
  const fs = await import("node:fs/promises");
  return {
    ...actual,
    execFile: (
      _exe: string,
      args: string[],
      options: { env?: NodeJS.ProcessEnv },
      callback: (error: Error | null, stdout: string, stderr: string) => void,
    ) => {
      observed.commands.push(args);
      observed.tokenInherited ||= Object.hasOwn(
        options.env ?? {},
        "NOUS_MEDIA_TEST_TOKEN",
      );
      if (args[0] === "-version") {
        callback(null, "ffmpeg version unit-process-fixture\n", "");
        return;
      }
      const input = args[args.indexOf("-i") + 1]!;
      observed.roots.push(input);
      if (args.at(-1) === "-") {
        callback(null, "", "Duration: 00:00:02.00\nStream #0:0 Video:");
        return;
      }
      if (observed.failFrames) {
        callback(new Error("fixture failure"), "", "ignored private error");
        return;
      }
      void fs
        .writeFile(args.at(-1)!, new Uint8Array([255, 216, 255, 217]))
        .then(
          () => callback(null, "", "showinfo pts_time:0"),
          () => callback(new Error("fixture write failure"), "", ""),
        );
    },
  };
});
import { sampleVideo } from "../src/model/video.js";
import { modelConfigurationSchema } from "../src/model/configuration.js";
it("bounds samples and subprocess input, excludes gateway credentials and removes temporary media on success and failure", async () => {
  const policy = modelConfigurationSchema.parse({
    video: {
      ffmpeg_executable: "fixture",
      max_frames: 3,
      max_video_seconds: 1,
    },
  }).video;
  process.env.NOUS_MEDIA_TEST_TOKEN = "fixture-secret";
  try {
    const result = await sampleVideo(
      new Uint8Array([1, 2, 3]),
      policy,
      ["nous_media_test_token"],
      false,
    );
    expect(result.frames).toHaveLength(3);
    expect(result.quality.sampled_timestamps).toEqual([0, 0.45, 0.9]);
    expect(result.quality.truncated).toBe(true);
    expect(observed.tokenInherited).toBe(false);
    expect(
      observed.commands.filter((args) => args.includes("-frames:v")),
    ).toHaveLength(3);
    for (const args of observed.commands.filter((entry) =>
      entry.includes("-i"),
    )) {
      expect(args[args.indexOf("-protocol_whitelist") + 1]).toBe("file,pipe");
      expect(args).toContain("-nostdin");
    }
    for (const input of observed.roots)
      await expect(stat(dirname(input))).rejects.toMatchObject({
        code: "ENOENT",
      });
    observed.failFrames = true;
    await expect(
      sampleVideo(new Uint8Array([1]), policy, [], false),
    ).rejects.toThrow("FFmpeg execution");
    for (const input of observed.roots)
      await expect(stat(dirname(input))).rejects.toMatchObject({
        code: "ENOENT",
      });
    await expect(
      sampleVideo(
        new Uint8Array(policy.max_source_bytes + 1),
        policy,
        [],
        false,
      ),
    ).rejects.toThrow("byte bound");
  } finally {
    delete process.env.NOUS_MEDIA_TEST_TOKEN;
  }
});
