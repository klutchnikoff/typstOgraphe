import { spawnSync } from "node:child_process";
import { existsSync, copyFileSync, readdirSync, unlinkSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const experiment = dirname(fileURLToPath(import.meta.url));
const repo = resolve(experiment, "../..");
process.chdir(repo);
function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`${command} exited ${result.status}`);
}
run("python3", [
  resolve(experiment, "prepare.py"),
  ...(process.argv[2] ? [resolve(process.argv[2])] : []),
]);
run("cargo", [
  "build",
  "--release",
  "--locked",
  "--offline",
  "--manifest-path",
  resolve(experiment, "Cargo.toml"),
]);
run("cargo", ["build", "--locked", "--offline", "-p", "typstographe"]);
const helper = resolve(repo, "crates/core/examples/yrs_archive_probe.rs");
if (existsSync(helper)) throw new Error(`Refusing to overwrite ${helper}`);
try {
  try {
    copyFileSync(resolve(experiment, "native-check.rs"), helper);
    run("cargo", [
      "build",
      "--locked",
      "--offline",
      "-p",
      "typstographe-core",
      "--example",
      "yrs_archive_probe",
    ]);
  } finally {
    if (existsSync(helper)) unlinkSync(helper);
  }
  run(resolve(repo, "node_modules/.bin/vitest"), [
    "run",
    "--config",
    resolve(experiment, "vitest.config.ts"),
  ]);
  run("python3", [resolve(experiment, "measure.py")]);
} finally {
  const artifacts = resolve(repo, "target/debug/examples");
  if (existsSync(artifacts)) {
    for (const file of readdirSync(artifacts)) {
      if (/^yrs_archive_probe(?:-[a-f0-9]+)?(?:\.d|\.exe)?$/.test(file))
        unlinkSync(resolve(artifacts, file));
    }
  }
}
