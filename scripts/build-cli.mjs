// Build the youskill CLI and place it where Tauri's `bundle.externalBin` expects it:
// src-tauri/binaries/youskill-<target-triple>[.exe]. Runs before `tauri dev` and
// `tauri build`. Set YOUSKILL_CLI_TARGET to cross-build for another triple (release CI).
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = join(root, "src-tauri", "Cargo.toml");
const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" })
  .split("\n")
  .find((line) => line.startsWith("host:"))
  .replace("host:", "")
  .trim();
const target = process.env.YOUSKILL_CLI_TARGET || host;
const release = process.env.YOUSKILL_CLI_PROFILE !== "debug";
const exe = target.includes("windows") ? ".exe" : "";

const args = ["build", "--manifest-path", manifest, "-p", "youskill-cli"];
if (release) args.push("--release");
if (target !== host) args.push("--target", target);
execFileSync("cargo", args, { stdio: "inherit" });

const profile = release ? "release" : "debug";
const built = join(
  root,
  "src-tauri",
  "target",
  ...(target !== host ? [target] : []),
  profile,
  `youskill${exe}`
);
const binaries = join(root, "src-tauri", "binaries");
if (!existsSync(binaries)) mkdirSync(binaries, { recursive: true });
const destination = join(binaries, `youskill-${target}${exe}`);
copyFileSync(built, destination);
console.log(`youskill -> ${destination}`);
