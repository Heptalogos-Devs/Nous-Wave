import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { bundleApplication } from "./bundle.js";
import { prepareNotices } from "./notices.js";
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const { packages } = await bundleApplication(repo);
await prepareNotices(repo, packages);
console.log("Release notices prepared");
