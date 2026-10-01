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
      if (attempt === 1) {
        const source =
          /^https:\/\/raw\.githubusercontent\.com\/([^/]+\/[^/]+)\/([^/]+)\/(.+)$/.exec(
            url,
          );
        if (!source) throw error;
        try {
          const response = await execute(
            "gh",
            [
              "api",
              `repos/${source[1]}/contents/${source[3]}?ref=${source[2]}`,
              "--jq",
              ".content",
            ],
            { windowsHide: true, maxBuffer: 2 * 1024 * 1024 },
          );
          const content = Buffer.from(
            response.stdout.replaceAll(/\s/g, ""),
            "base64",
          ).toString("utf8");
          await mkdir(dirname(cache), { recursive: true });
          await writeFile(cache, content);
          return content;
        } catch (fallback) {
          if (
            fallback instanceof Error &&
            "stderr" in fallback &&
            String(fallback.stderr).includes("HTTP 404")
          )
            return undefined;
          throw error;
        }
      }
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
  join(repo, "target/x86_64-pc-windows-gnullvm/release/nous-kernel.exe"),
  join(program, "kernel/nous-kernel.exe"),
);
for (const library of ["libc++.dll", "libunwind.dll"]) {
  await cp(
    join(repo, "target/x86_64-pc-windows-gnullvm/release", library),
    join(program, "kernel", library),
  );
}
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
const kernelClosure = await kernelInventory(repo, output);
if (kernelClosure.missingNotices.length)
  throw new Error(
    "Kernel license text missing: " + kernelClosure.missingNotices.join(", "),
  );
const runtimeComponents: Component[] = [];
const nativeRelationships: Relationship[] = [];
for (const pack of components.packs) {
  const runtimeManifest = JSON.parse(
    await readFile(join(runtime, pack.component, "manifest.json"), "utf8"),
  ) as { license: string; source: string };
  runtimeComponents.push({
    id: `SPDXRef-runtime-${pack.component}`,
    name: pack.component,
    version: pack.version,
    license: runtimeManifest.license,
    source: runtimeManifest.source,
    purpose: "APPLICATION",
  });
}
const nodeVersions = JSON.parse(
  (
    await execute(
      join(runtime, "node/node.exe"),
      ["-p", "JSON.stringify(process.versions)"],
      { windowsHide: true },
    )
  ).stdout,
) as Record<string, string>;
const nodeLicensePath = "runtime/node/licenses/node/LICENSE";
const nodeLicense = await readFile(join(output, nodeLicensePath), "utf8");
const nodeDependencies = [
  ["acorn", "Acorn", "deps/acorn", "MIT"],
  ["ada", "ada", "deps/ada", "MIT"],
  ["amaro", "amaro", "deps/amaro", "MIT"],
  ["ares", "c-ares", "deps/cares", "MIT"],
  ["brotli", "brotli", "deps/brotli", "MIT"],
  ["icu", "ICU", "deps/icu-small", "Unicode-3.0"],
  ["llhttp", "llhttp", "deps/llhttp", "MIT"],
  ["merve", "merve", "deps/merve", "MIT"],
  ["nghttp2", "nghttp2", "deps/nghttp2", "MIT"],
  ["openssl", "OpenSSL", "deps/openssl", "Apache-2.0"],
  ["simdjson", "simdjson", "deps/simdjson", "Apache-2.0"],
  ["simdutf", "simdutf", "deps/v8/third_party/simdutf", "MIT"],
  ["undici", "undici", "deps/undici", "MIT"],
  ["uv", "libuv", "deps/uv", "MIT"],
  ["uvwasi", "uvwasi", "deps/uvwasi", "MIT"],
  ["v8", "V8", "deps/v8", "BSD-3-Clause"],
  ["zlib", "zlib", "deps/zlib", "Zlib"],
  ["zstd", "zstd", "deps/zstd", "BSD-3-Clause"],
] as const;
for (const [key, name, sourcePath, license] of nodeDependencies) {
  if (
    !nodeVersions[key] ||
    !nodeLicense.includes(`- ${name}, located at ${sourcePath},`)
  )
    throw new Error(`Node embedded dependency version/notice missing: ${name}`);
  const id = `SPDXRef-node-embedded-${key}`;
  runtimeComponents.push({
    id,
    name,
    version: nodeVersions[key],
    license,
    source: `https://github.com/nodejs/node/tree/v${nodeVersions.node}/${sourcePath}`,
    purpose: "LIBRARY",
    notice: nodeLicensePath,
  });
  nativeRelationships.push({
    spdxElementId: "SPDXRef-runtime-node",
    relationshipType: "CONTAINS",
    relatedSpdxElement: id,
  });
}
for (const [key, sourcePath, license] of [
  ["nbytes", "deps/nbytes/LICENSE", "MIT"],
  ["ncrypto", "LICENSE", "MIT"],
  ["sqlite", "deps/sqlite/sqlite3.h", "LicenseRef-SQLite-Public-Domain"],
] as const) {
  const source = `https://raw.githubusercontent.com/nodejs/node/v${nodeVersions.node}/${sourcePath}`;
  const content = await publicFile(source);
  if (!nodeVersions[key] || !content)
    throw new Error(`Node notice unavailable: ${key}`);
  const notice = `licenses/node-embedded/${key}.txt`;
  await mkdir(join(output, "licenses/node-embedded"), { recursive: true });
  await writeFile(
    join(output, notice),
    key === "sqlite" ? content.split("*/", 1)[0] + "*/\n" : content,
  );
  const id = `SPDXRef-node-embedded-${key}`;
  runtimeComponents.push({
    id,
    name: key,
    version: nodeVersions[key],
    license,
    source,
    purpose: "LIBRARY",
    notice,
  });
  nativeRelationships.push({
    spdxElementId: "SPDXRef-runtime-node",
    relationshipType: "CONTAINS",
    relatedSpdxElement: id,
  });
}
const compilerRoot = join(
  repo,
  "data/runtime-build/llvm-mingw-windows/llvm-mingw-20260922-ucrt-x86_64",
);
await mkdir(join(output, "licenses/native"), { recursive: true });
await cp(
  join(compilerRoot, "LICENSE.TXT"),
  join(output, "licenses/native/LLVM-LICENSE.txt"),
);
await cp(
  join(runtime, "postgresql/licenses/mingw-runtime"),
  join(output, "licenses/native/mingw-runtime"),
  { recursive: true },
);
const rustSysroot = (
  await execute("rustc", ["--print", "sysroot"], { windowsHide: true })
).stdout.trim();
const rustVersion = (await execute("rustc", ["-Vv"], { windowsHide: true }))
  .stdout;
await cp(
  join(rustSysroot, "share/doc/rust/COPYRIGHT-library.html"),
  join(output, "licenses/native/Rust-library-COPYRIGHT.html"),
);
await cp(
  join(rustSysroot, "share/doc/rust/licenses"),
  join(output, "licenses/native/rust"),
  { recursive: true },
);
await writeFile(join(output, "licenses/native/rust-source.txt"), rustVersion);
const rustRelease = /^release: (.+)$/m.exec(rustVersion)?.[1];
const rustCommit = /^commit-hash: (.+)$/m.exec(rustVersion)?.[1];
if (!rustRelease || !rustCommit)
  throw new Error("Rust library provenance unavailable");
for (const component of [
  {
    id: "SPDXRef-native-llvm-runtime",
    name: "LLVM runtime (libc++/compiler-rt/libunwind)",
    version: "20260922",
    license: "Apache-2.0 WITH LLVM-exception",
    source: "https://github.com/mstorsjo/llvm-mingw/releases/tag/20260922",
    purpose: "LIBRARY",
    notice: "licenses/native/LLVM-LICENSE.txt",
  },
  {
    id: "SPDXRef-native-mingw-runtime",
    name: "MinGW-w64 runtime",
    version: "57b595039040eaa15bece85b7cc71d952281b269",
    license: "LicenseRef-MinGW-Runtime",
    source:
      "https://github.com/mingw-w64/mingw-w64/tree/57b595039040eaa15bece85b7cc71d952281b269",
    purpose: "LIBRARY",
    notice: "licenses/native/mingw-runtime",
  },
  {
    id: "SPDXRef-native-rust-library",
    name: "Rust standard library",
    version: rustRelease,
    license: "MIT OR Apache-2.0",
    source: `https://github.com/rust-lang/rust/tree/${rustCommit}/library`,
    purpose: "LIBRARY",
    notice: "licenses/native/Rust-library-COPYRIGHT.html",
  },
] satisfies Component[]) {
  runtimeComponents.push(component);
  nativeRelationships.push({
    spdxElementId: kernelClosure.root,
    relationshipType: "DEPENDS_ON",
    relatedSpdxElement: component.id,
  });
}
for (const name of ["postgresql", "ffmpeg"])
  for (const dependency of ["llvm", "mingw"])
    nativeRelationships.push({
      spdxElementId: `SPDXRef-runtime-${name}`,
      relationshipType: "DEPENDS_ON",
      relatedSpdxElement: `SPDXRef-native-${dependency}-runtime`,
    });
await mkdir(join(output, "manifest"));
await writeFile(
  join(output, "manifest/components.json"),
  JSON.stringify(
    {
      application: inventory,
      kernel: kernelClosure.components,
      missing_cargo_notices: kernelClosure.missingNotices,
      runtimes: components.packs,
    },
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
      hasExtractedLicensingInfos: [
        {
          licenseId: "LicenseRef-MinGW-Runtime",
          name: "MinGW-w64 runtime composite notices",
          extractedText: await readFile(
            join(
              output,
              "licenses/native/mingw-runtime/COPYING.MinGW-w64-runtime.txt",
            ),
            "utf8",
          ),
          seeAlsos: [
            "https://github.com/mingw-w64/mingw-w64/tree/57b595039040eaa15bece85b7cc71d952281b269",
          ],
        },
        {
          licenseId: "LicenseRef-SQLite-Public-Domain",
          name: "SQLite public domain dedication",
          extractedText:
            "The author disclaims copyright to this source code. See the retained SQLite header for its full dedication.",
          seeAlsos: [
            `https://raw.githubusercontent.com/nodejs/node/v${nodeVersions.node}/deps/sqlite/sqlite3.h`,
          ],
        },
      ],
      packages: [
        ...inventory.map((pkg, index) =>
          spdxPackage({
            id: `SPDXRef-npm-${index}`,
            ...pkg,
            source: `https://registry.npmjs.org/${pkg.name}/-/${pkg.name.split("/").at(-1)}-${pkg.version}.tgz`,
            purpose: "LIBRARY",
          }),
        ),
        ...kernelClosure.components.map(spdxPackage),
        ...runtimeComponents.map(spdxPackage),
      ],
      relationships: [
        ...kernelClosure.relationships,
        ...nativeRelationships,
        ...[
          kernelClosure.root,
          ...runtimeComponents.map((component) => component.id),
        ].map((id) => ({
          spdxElementId: "SPDXRef-DOCUMENT",
          relationshipType: "DESCRIBES",
          relatedSpdxElement: id,
        })),
      ],
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

type CargoPackage = {
  id: string;
  name: string;
  version: string;
  license: string | null;
  license_file: string | null;
  manifest_path: string;
  source: string | null;
  repository: string | null;
};
type CargoNode = {
  id: string;
  deps: { pkg: string; dep_kinds: { kind: string | null }[] }[];
};
type Component = {
  id: string;
  name: string;
  version: string;
  license: string;
  source: string;
  purpose: "APPLICATION" | "LIBRARY" | "OTHER";
  notice?: string;
};
type Relationship = {
  spdxElementId: string;
  relationshipType: string;
  relatedSpdxElement: string;
};

/** The release graph excludes test-only edges and records build inputs explicitly. */
async function kernelInventory(repositoryRoot: string, bundleRoot: string) {
  const executeCargo = promisify(execFile);
  const metadata = JSON.parse(
    (
      await executeCargo(
        "cargo",
        [
          "metadata",
          "--locked",
          "--format-version",
          "1",
          "--filter-platform",
          "x86_64-pc-windows-gnullvm",
        ],
        { cwd: repositoryRoot, windowsHide: true, maxBuffer: 16 * 1024 * 1024 },
      )
    ).stdout,
  ) as {
    packages: CargoPackage[];
    resolve: { nodes: CargoNode[] };
  };
  const cargoPackages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const kernel = metadata.packages.find(
    (pkg) => pkg.name === "nous-kernel" && !pkg.source,
  );
  if (!kernel) throw new Error("Kernel release dependency root missing");
  const selected = new Set<string>();
  const buildOnly = new Set<string>();
  const pending = [{ id: kernel.id, build: false }];
  const walked = new Set<string>();
  while (pending.length) {
    const item = pending.pop()!;
    const traversal = `${item.id}:${item.build}`;
    if (walked.has(traversal)) continue;
    walked.add(traversal);
    if (!selected.has(item.id) && item.build) buildOnly.add(item.id);
    if (!item.build) buildOnly.delete(item.id);
    selected.add(item.id);
    for (const dep of nodes.get(item.id)?.deps ?? []) {
      for (const kind of dep.dep_kinds) {
        if (kind.kind === "dev") continue;
        pending.push({
          id: dep.pkg,
          build: item.build || kind.kind === "build",
        });
      }
    }
  }
  const ordered = [...selected].sort();
  const ids = new Map(
    ordered.map((id, index) => [id, `SPDXRef-cargo-${index}`]),
  );
  const cargoComponents: Component[] = [];
  const relationships: Relationship[] = [];
  const missingNotices: string[] = [];
  for (const id of ordered) {
    const pkg = cargoPackages.get(id);
    if (!pkg) throw new Error("Release dependency package missing");
    const root = dirname(pkg.manifest_path);
    const destination = join(
      bundleRoot,
      "licenses/cargo",
      pkg.name + "-" + pkg.version,
    );
    await mkdir(destination, { recursive: true });
    await cp(pkg.manifest_path, join(destination, "Cargo.toml"));
    const names = (await readdir(root)).filter((name) =>
      /^(?:license|copying|notice|readme)(?:[._-]|$)/i.test(name),
    );
    if (pkg.license_file && !names.includes(pkg.license_file))
      names.push(pkg.license_file);
    let notices = 0;
    for (const name of names) {
      const content = await readFile(join(root, name)).catch(
        (error: NodeJS.ErrnoException) => {
          if (error.code === "EISDIR") return undefined;
          throw error;
        },
      );
      if (!content) continue;
      await writeFile(
        join(destination, name.replaceAll(/[\\/]/g, "_")),
        content,
      );
      if (
        /^(?:license|copying|notice)/i.test(name) ||
        name === pkg.license_file
      )
        notices++;
    }
    if (!pkg.source) {
      await cp(join(repositoryRoot, "LICENSE"), join(destination, "LICENSE"));
      notices++;
    }
    if (!notices && pkg.source?.startsWith("git+")) {
      let parent = dirname(root);
      for (let depth = 0; depth < 3 && !notices; depth++) {
        for (const name of [
          "LICENSE",
          "LICENSE-MIT",
          "LICENSE-APACHE",
          "NOTICE",
        ]) {
          const content = await readFile(join(parent, name)).catch(
            (error: NodeJS.ErrnoException) => {
              if (error.code === "ENOENT") return undefined;
              throw error;
            },
          );
          if (!content) continue;
          await writeFile(join(destination, name), content);
          await writeFile(
            join(destination, name + ".source.txt"),
            pkg.source + "\n",
          );
          if (name !== "NOTICE") notices++;
        }
        parent = dirname(parent);
      }
    }
    if (!notices && pkg.repository) {
      const vcs = await readFile(
        join(root, ".cargo_vcs_info.json"),
        "utf8",
      ).catch(() => undefined);
      const revision = vcs
        ? (JSON.parse(vcs) as { git: { sha1: string } }).git.sha1
        : undefined;
      if (revision && /^[a-f0-9]{40}$/.test(revision)) {
        const repository = /github\.com[/:]([^/]+\/[^/]+?)(?:\.git)?\/?$/.exec(
          pkg.repository,
        )?.[1];
        if (repository) {
          for (const name of [
            "LICENSE",
            "LICENSE-MIT",
            "LICENSE-APACHE",
            "LICENSE.txt",
            "LICENSES/Apache-2.0.txt",
            "LICENSES/MIT.txt",
            "NOTICE",
          ]) {
            const url = `https://raw.githubusercontent.com/${repository}/${revision}/${name}`;
            const notice = await publicFile(url);
            if (notice === undefined) continue;
            const filename = name.replaceAll("/", "_");
            await writeFile(join(destination, filename), notice);
            await writeFile(
              join(destination, filename + ".source.txt"),
              url + "\n",
            );
            if (name !== "NOTICE") notices++;
          }
        }
      }
    }
    if (!notices && pkg.name === "htmlescape" && pkg.version === "0.3.1") {
      // The published crate grants Apache/MIT/MPL alternatives; its old repository is unavailable.
      const source = "https://www.apache.org/licenses/LICENSE-2.0.txt";
      const license = await publicFile(source);
      if (!license) throw new Error("Apache license text unavailable");
      await writeFile(join(destination, "LICENSE-APACHE"), license);
      await writeFile(
        join(destination, "license-choice.txt"),
        "Distributed under the Apache-2.0 option declared by the publisher in the retained Cargo.toml. " +
          "Publisher metadata and README are retained; no additional NOTICE was present in the published crate.\n" +
          source +
          "\n",
      );
      notices++;
    }
    if (!notices) missingNotices.push(pkg.name + "@" + pkg.version);
    cargoComponents.push({
      id: ids.get(id)!,
      name: pkg.name,
      version: pkg.version,
      license: pkg.license?.replaceAll(/\s*\/\s*/g, " OR ") ?? "NOASSERTION",
      source: pkg.source?.startsWith("registry+")
        ? `https://crates.io/api/v1/crates/${pkg.name}/${pkg.version}/download`
        : (pkg.source ?? "NOASSERTION"),
      purpose: buildOnly.has(id)
        ? "OTHER"
        : pkg.name === "nous-kernel"
          ? "APPLICATION"
          : "LIBRARY",
    });
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (!selected.has(dep.pkg)) continue;
      const kinds = new Set(
        dep.dep_kinds
          .filter((kind) => kind.kind !== "dev")
          .map((kind) => kind.kind),
      );
      for (const kind of kinds)
        relationships.push({
          spdxElementId: ids.get(kind === "build" ? dep.pkg : id)!,
          relationshipType:
            kind === "build" ? "BUILD_DEPENDENCY_OF" : "DEPENDS_ON",
          relatedSpdxElement: ids.get(kind === "build" ? id : dep.pkg)!,
        });
    }
  }
  return {
    components: cargoComponents,
    relationships,
    missingNotices,
    root: ids.get(kernel.id)!,
  };
}

function spdxPackage(component: Component) {
  return {
    SPDXID: component.id,
    name: component.name,
    versionInfo: component.version,
    downloadLocation: component.source,
    filesAnalyzed: false,
    licenseDeclared: component.license,
    licenseConcluded: "NOASSERTION",
    copyrightText: "NOASSERTION",
    primaryPackagePurpose: component.purpose,
    ...(component.notice
      ? {
          attributionTexts: [
            `License and attribution retained at ${component.notice}`,
          ],
        }
      : {}),
  };
}
