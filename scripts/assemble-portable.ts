import { build } from "esbuild";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import {
  cp,
  mkdir,
  readFile,
  readdir,
  stat,
  writeFile,
} from "node:fs/promises";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify, parseArgs } from "node:util";
import { execFile } from "node:child_process";
const execute = promisify(execFile);
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({ options: { output: { type: "string" } } });
if (!values.output || process.platform !== "win32" || process.arch !== "x64")
  throw new Error("Provide a fresh Windows x64 --output directory");
const output = resolve(values.output);
if (await stat(output).catch(() => undefined))
  throw new Error("Portable output already exists");
await mkdir(output, { recursive: true });
const program = join(output, "program"),
  runtime = join(output, "runtime");
for (const folder of ["core", "cli", "client", "kernel", "manifest"])
  await mkdir(join(program, folder), { recursive: true });
await writeFile(join(program, "package.json"), '{"type":"module"}\n');
const entries = [
  ["apps/nous-core/src/main.ts", "core/main.js"],
  ["apps/nous-core/src/launcher.ts", "core/launcher.js"],
  ["apps/nous-cli/src/main.ts", "cli/main.js"],
  ["packages/client/src/index.ts", "client/index.js"],
  ["packages/client/src/node.ts", "client/node.js"],
] as const;
const packages = new Map<
  string,
  { name: string; version: string; license: string; root: string }
>();
async function publicFile(url: string) {
  const key = createHash("sha256").update(url).digest("hex");
  const cache = join(repo, "data", "runtime-build", "license-cache", key);
  const cached = await readFile(cache, "utf8").catch(() => undefined);
  if (cached !== undefined) return cached;
  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      const response = await fetch(url, {
        signal: AbortSignal.timeout(30000),
        headers: { connection: "close" },
      });
      if (response.status === 404) return undefined;
      if (!response.ok) throw new Error("Public notice fetch failed");
      const content = await response.text();
      await mkdir(dirname(cache), { recursive: true });
      await writeFile(cache, content);
      return content;
    } catch (error) {
      if (attempt === 1) throw error;
    }
  }
  throw new Error("Public notice unavailable");
}
for (const [entry, target] of entries) {
  const result = await build({
    absWorkingDir: repo,
    entryPoints: [entry],
    outfile: join(program, target),
    bundle: true,
    platform: "node",
    target: "node24",
    format: "esm",
    metafile: true,
    minify: true,
    banner: {
      js: 'import { createRequire as __createRequire } from "node:module"; import { fileURLToPath as __fileURLToPath } from "node:url"; import { dirname as __dirnameOf } from "node:path"; const require=__createRequire(import.meta.url); const __filename=__fileURLToPath(import.meta.url); const __dirname=__dirnameOf(__filename);',
    },
    logLevel: "warning",
  });
  for (const input of Object.keys(result.metafile!.inputs)) {
    const marker = input.lastIndexOf("node_modules/");
    if (marker < 0) continue;
    const tail = input.slice(marker + 13).split("/");
    const count = tail[0]?.startsWith("@") ? 2 : 1;
    const root = resolve(
      repo,
      input.slice(0, marker + 13) + tail.slice(0, count).join("/"),
    );
    const metadata = JSON.parse(
      await readFile(join(root, "package.json"), "utf8"),
    ) as { name: string; version: string; license?: string };
    packages.set(metadata.name + "@" + metadata.version, {
      name: metadata.name,
      version: metadata.version,
      license: metadata.license ?? "NOASSERTION",
      root,
    });
  }
}
await cp(
  join(repo, "target/release/nous-kernel.exe"),
  join(program, "kernel/nous-kernel.exe"),
);
await cp(join(repo, "prompts"), join(program, "prompts"), { recursive: true });
await mkdir(join(program, "templates"));
await cp(join(repo, "nous.example.toml"), join(program, "templates/nous.toml"));
await cp(
  join(repo, "crates/persistence/migrations"),
  join(program, "migrations"),
  { recursive: true },
);
await cp(
  join(repo, "manifest/runtimes.json"),
  join(program, "manifest/runtimes.json"),
);
await mkdir(runtime);
const components = JSON.parse(
  await readFile(join(repo, "manifest/runtimes.json"), "utf8"),
) as {
  packs: {
    component: string;
    version: string;
    platform: string;
    arch: string;
    sha256: string;
    manifest_sha256: string;
    archive: string;
  }[];
};
for (const name of ["node", "postgresql", "ffmpeg"]) {
  const matching = components.packs.filter(
    (pack) =>
      pack.component === name &&
      pack.platform === process.platform &&
      pack.arch === process.arch,
  );
  if (matching.length !== 1)
    throw new Error(`No unique ${name} pack for the bundle platform`);
  await execute(
    process.execPath,
    [
      join(program, "core/launcher.js"),
      "runtime",
      "install",
      name,
      "--home",
      output,
      "--pack",
      join(repo, "packs", matching[0]!.archive),
    ],
    { windowsHide: true, maxBuffer: 65536 },
  );
}
await mkdir(join(output, "config"));
await cp(join(repo, "nous.example.toml"), join(output, "config/nous.toml"));
await cp(join(repo, "bootstrap.example.toml"), join(output, "bootstrap.toml"));
await mkdir(join(output, "bin"));
await writeFile(
  join(output, "bin/nous.cmd"),
  '@echo off\r\n"%~dp0..\\runtime\\node\\node.exe" "%~dp0..\\program\\core\\launcher.js" %*\r\n',
);
await mkdir(join(output, "licenses/npm"), { recursive: true });
await cp(join(repo, "LICENSE"), join(output, "licenses/Nous-Wave-MIT.txt"));
for (const pkg of packages.values()) {
  const destination = join(
    output,
    "licenses/npm",
    pkg.name.replace(/[\\/@]/g, "_") + "-" + pkg.version,
  );
  await mkdir(destination);
  const notices = (await readdir(pkg.root)).filter((name) =>
    /^(licen[cs]e|copying|notice)(\.|$)/i.test(name),
  );
  if (!notices.length) {
    const metadata = JSON.parse(
      await readFile(join(pkg.root, "package.json"), "utf8"),
    ) as { repository?: string | { url?: string } };
    const repository =
      typeof metadata.repository === "string"
        ? metadata.repository
        : metadata.repository?.url;
    const match =
      repository && /github\.com[/:]([^/]+\/[^/.]+)(?:\.git)?/.exec(repository);
    if (!match)
      throw new Error(
        "Bundled dependency has no traceable license source: " + pkg.name,
      );
    let base = `https://raw.githubusercontent.com/${match[1]}/v${pkg.version}/`;
    let licenseSource = base + "LICENSE";
    let response = await publicFile(licenseSource);
    if (response === undefined) {
      base = `https://raw.githubusercontent.com/${match[1]}/${encodeURI(pkg.name + "@" + pkg.version)}/`;
      licenseSource = base + "LICENSE";
      response = await publicFile(licenseSource);
    }
    if (response !== undefined)
      await writeFile(join(destination, "LICENSE"), response);
    else {
      const readmeName = (await readdir(pkg.root)).find((name) =>
        /^readme\.md$/i.test(name),
      );
      const readme = readmeName
        ? await readFile(join(pkg.root, readmeName), "utf8")
        : "";
      const linked = /\[(?:MIT )?License\]\((https?:\/\/[^)]+)\)/i.exec(readme);
      if (!linked)
        throw new Error(
          "Version-specific license source unavailable: " + pkg.name,
        );
      const url = new URL(linked[1]!);
      url.protocol = "https:";
      licenseSource = url.toString();
      const license = await publicFile(licenseSource);
      if (license === undefined)
        throw new Error("Publisher license source unavailable: " + pkg.name);
      await writeFile(join(destination, "LICENSE.html"), license);
      await writeFile(join(destination, "README-license-source.md"), readme);
    }
    const notice = await publicFile(base + "NOTICE");
    if (notice !== undefined)
      await writeFile(join(destination, "NOTICE"), notice);
    await writeFile(
      join(destination, "source.json"),
      JSON.stringify({
        license_source: licenseSource,
        package: pkg.name,
        version: pkg.version,
      }) + "\n",
    );
  }
  for (const name of notices)
    if ((await stat(join(pkg.root, name))).isFile())
      await cp(join(pkg.root, name), join(destination, name));
}
const inventory = [...packages.values()].map(({ name, version, license }) => ({
  name,
  version,
  license,
}));
await mkdir(join(output, "manifest"));
await writeFile(
  join(output, "manifest/components.json"),
  JSON.stringify(
    { application: inventory, runtimes: components.packs },
    null,
    2,
  ) + "\n",
);
await writeFile(
  join(output, "licenses/THIRD_PARTY_NOTICES.md"),
  "# Third-party components\n\nApplication dependencies and runtime inventories are in manifest/components.json. Runtime-specific licenses, exact source and build references are retained inside runtime/<component>/licenses. Nous Wave-owned code is MIT.\n",
);
await writeFile(
  join(output, "manifest/sbom.spdx.json"),
  JSON.stringify(
    {
      spdxVersion: "SPDX-2.3",
      dataLicense: "CC0-1.0",
      SPDXID: "SPDXRef-DOCUMENT",
      name: "Nous-Wave-runtime-bundle",
      documentNamespace:
        "https://nous-wave.invalid/spdx/" + crypto.randomUUID(),
      creationInfo: {
        creators: ["Tool: Nous-Wave-assembler"],
        created: new Date().toISOString(),
      },
      packages: inventory.map((pkg, index) => ({
        SPDXID: "SPDXRef-npm-" + index,
        name: pkg.name,
        versionInfo: pkg.version,
        downloadLocation: "NOASSERTION",
        filesAnalyzed: false,
        licenseDeclared: pkg.license,
        licenseConcluded: "NOASSERTION",
        copyrightText: "NOASSERTION",
      })),
    },
    null,
    2,
  ) + "\n",
);
const checksums: { path: string; sha256: string }[] = [];
async function collect(directory: string) {
  for (const file of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, file.name);
    if (file.isDirectory()) await collect(path);
    else {
      const hash = createHash("sha256");
      for await (const bytes of createReadStream(path))
        hash.update(bytes as Buffer);
      checksums.push({
        path: path
          .slice(output.length + 1)
          .split(sep)
          .join("/"),
        sha256: hash.digest("hex"),
      });
    }
  }
}
await collect(output);
await writeFile(
  join(output, "manifest/checksums.json"),
  JSON.stringify(checksums, null, 2) + "\n",
);
const head = (
  await execute("git", ["rev-parse", "HEAD"], { cwd: repo })
).stdout.trim();
const sourceFiles = (
  await execute(
    "git",
    ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
    { cwd: repo },
  )
).stdout
  .split("\0")
  .filter(Boolean)
  .sort();
const sourceHash = createHash("sha256");
for (const path of sourceFiles) {
  const contents = await readFile(join(repo, path)).catch(
    (error: NodeJS.ErrnoException) => {
      if (error.code === "ENOENT") return undefined;
      throw error;
    },
  );
  if (contents === undefined) continue;
  sourceHash
    .update(path)
    .update("\0")
    .update(createHash("sha256").update(contents).digest())
    .update("\0");
}
const sourceInputDigest = sourceHash.digest("hex");
const sourceDirty = Boolean(
  (
    await execute("git", ["status", "--porcelain"], { cwd: repo })
  ).stdout.trim(),
);
await writeFile(
  join(output, "manifest/release.json"),
  JSON.stringify(
    {
      name: "Nous Wave",
      version: "0.1.0",
      platform: process.platform,
      arch: process.arch,
      source_head: head,
      source_input_sha256: sourceInputDigest,
      source_dirty: sourceDirty,
      source_tree: "working-tree-candidate",
      launcher: "bin/nous.cmd",
      application_license: "MIT",
      status: "NOT_RUN",
    },
    null,
    2,
  ) + "\n",
);
const systemRoot = process.env.SystemRoot;
if (!systemRoot) throw new Error("SystemRoot missing");
await execute(
  join(systemRoot, "System32/WindowsPowerShell/v1.0/powershell.exe"),
  [
    "-NoProfile",
    "-File",
    join(repo, "scripts/zip-runtime.ps1"),
    "-SourceRoot",
    output,
    "-ArchivePath",
    output + ".zip",
  ],
  { windowsHide: true, maxBuffer: 65536 },
);
console.log(
  JSON.stringify({
    bundle: output + ".zip",
    status: "NOT_RUN",
    source_head: head,
  }),
);
