// Fixture orchestration only; both storage engines and timings are native Rust.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const outputFlag = process.argv.indexOf("--output");
const output =
  outputFlag < 0
    ? resolve(root, "output/a4-storage-evaluation")
    : resolve(process.argv[outputFlag + 1]);
const cli = resolve(
  root,
  `target/debug/typstographe-cli${process.platform === "win32" ? ".exe" : ""}`,
);
function run(program, args, options = {}) {
  const result = spawnSync(program, args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(
      `${program} ${args.join(" ")}\n${result.stderr}\n${result.stdout}`,
    );
  return result.stdout;
}
const sha = (data) => createHash("sha256").update(data).digest("hex");
const revision = run("git", ["rev-parse", "HEAD"]).trim();
mkdirSync(output, { recursive: true });
run("cargo", ["build", "--locked", "--offline", "-p", "typstographe-cli"]);

// Reproducible noisy PNGs model compressed binary assets, not easily compressed SVGs.
function png(seed) {
  function crc32(data) {
    let crc = 0xffffffff;
    for (const byte of data) {
      crc ^= byte;
      for (let bit = 0; bit < 8; bit++)
        crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
    }
    return (crc ^ 0xffffffff) >>> 0;
  }
  function chunk(kind, data) {
    const name = Buffer.from(kind),
      length = Buffer.alloc(4),
      crc = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    crc.writeUInt32BE(crc32(Buffer.concat([name, data])));
    return Buffer.concat([length, name, data, crc]);
  }
  const width = 512,
    height = 512,
    header = Buffer.alloc(13);
  header.writeUInt32BE(width);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 2;
  const pixels = Buffer.alloc(height * (width * 3 + 1));
  let state = seed;
  for (let row = 0; row < height; row++)
    for (let col = 1; col <= width * 3; col++) {
      state ^= state << 13;
      state ^= state >>> 17;
      state ^= state << 5;
      pixels[row * (width * 3 + 1) + col] = state & 255;
    }
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(pixels)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}
const images = [png(17), png(29), png(43)].map((data) => ({
  path: `images/${sha(data).slice(0, 24)}.png`,
  data: data.toString("base64"),
}));
const para = (text) => ({
  type: "paragraph",
  content: [{ type: "text", text }],
});
const figure = (resource) => ({
  type: "figure",
  attrs: { src: resource.path, alt: "Deterministic storage evaluation image" },
  content: [{ type: "text", text: "Storage evaluation fixture" }],
});
const parent = { root: { kind: "body" } };
const call = (id, params) => ({ id, params });
const marker = (i) =>
  `Révision ${String(i).padStart(3, "0")} — conservation du texte, des accents é et 日本語.`;
function script(document, steps, destination) {
  const result = JSON.parse(
    run(cli, ["--json", "--lang", "en", "script", "-"], {
      input: JSON.stringify({
        version: 1,
        document,
        steps,
        save: { kind: "as", path: destination },
      }),
    }),
  );
  return result;
}
const sourceArticle = resolve(output, "committed-article.typsto");
const articleBytes = run(
  "git",
  ["show", `${revision}:examples/article-working.typsto`],
  { encoding: null },
);
writeFileSync(sourceArticle, articleBytes);
const letterScript = JSON.parse(
  readFileSync(resolve(root, "examples/cli/courrier.json"), "utf8"),
);
const corpora = [];
for (const [name, count, expanded] of [
  ["courrier", 100, false],
  ["article", 100, false],
  ["article-long", 100, true],
]) {
  const folder = resolve(output, name);
  mkdirSync(folder, { recursive: true });
  const steps = name === "courrier" ? letterScript.steps.slice(0, -1) : [];
  const nodes = [para(marker(0)), figure(images[0])];
  if (expanded)
    for (let i = 0; i < 250; i++)
      nodes.push(
        para(
          `Paragraphe ${i + 1}. Cette observation porte sur les pratiques de rédaction et leur évolution. Les résultats sont discutés dans leur contexte institutionnel, avec des limites explicites et des références conservées.`,
        ),
      );
  steps.push(
    call("resource.add", { resource: images[0] }),
    call("block.splice", { parent, index: 0, delete_count: 0, nodes }),
  );
  const initial = resolve(folder, "000.typsto");
  if (!existsSync(initial))
    script(
      name === "courrier"
        ? { kind: "new", type: "courrier" }
        : { kind: "open", path: sourceArticle },
      steps,
      initial,
    );
  // Query the actual model source through the public document inspection contract.
  const inspected = JSON.parse(
    run(cli, ["--json", "--lang", "en", "query", "document.inspect", initial]),
  );
  const settings = inspected.outcome?.state?.settings;
  if (!settings) throw new Error("Inspection did not expose model settings");
  let template = settings.template;
  for (let i = 1; i < count; i++) {
    const edits = [
      call("text.insert", {
        target: {
          zone: { kind: "body", path: [0] },
          from: 0,
          to: marker(i - 1).length,
        },
        text: marker(i),
      }),
    ];
    if (i === 10)
      edits.push(
        call("resource.add", { resource: images[1] }),
        call("block.insert", { parent, index: 2, node: figure(images[1]) }),
      );
    if (i === 20)
      edits.push(
        call("resource.add", { resource: images[2] }),
        call("node.set_attrs", {
          target: { ...parent, path: [2] },
          attrs: { src: images[2].path },
        }),
      );
    if (i === 30)
      edits.push(call("block.remove", { target: { ...parent, path: [2] } }));
    if (i === 40 || i === 70) {
      template += `\n// Storage evaluation: pinned model revision ${i}.\n`;
      edits.push(
        call("document.set_model", {
          model_id: settings.modelId,
          source: template,
        }),
      );
    }
    const file = resolve(folder, `${String(i).padStart(3, "0")}.typsto`);
    if (!existsSync(file))
      script(
        {
          kind: "open",
          path: resolve(folder, `${String(i - 1).padStart(3, "0")}.typsto`),
        },
        edits,
        file,
      );
    if (i % 20 === 0)
      process.stderr.write(`${name}: ${i}/${count} fixture revisions\n`);
  }
  corpora.push({ name, directory: folder, count, expanded });
}
const environment = {
  revision,
  platform: process.platform,
  arch: process.arch,
  node: process.version,
  rust: run("rustc", ["--version"]).trim(),
  machine:
    process.platform === "darwin"
      ? run("sysctl", ["-n", "machdep.cpu.brand_string"]).trim()
      : "See host platform",
  article_sha256: sha(articleBytes),
  runner_sha256: sha(readFileSync(fileURLToPath(import.meta.url))),
  probe_sha256: sha(readFileSync(resolve(here, "src/main.rs"))),
  image_bytes: images.map((image) => Buffer.from(image.data, "base64").length),
  corpora,
};
writeFileSync(
  resolve(output, "corpus.json"),
  JSON.stringify(environment, null, 2) + "\n",
);
if (process.argv.includes("--fixtures-only")) process.exit(0);
process.stderr.write(
  "Fixtures complete; building isolated release storage benchmark.\n",
);
run("cargo", [
  "build",
  "--release",
  "--locked",
  "--offline",
  "--manifest-path",
  resolve(here, "Cargo.toml"),
]);
const binary = resolve(
  here,
  `target/release/typstographe-snapshot-storage-evaluation${process.platform === "win32" ? ".exe" : ""}`,
);
run(binary, [output], { stdio: "inherit" });
// Reconstructed projects must still pass the current core and offline Typst compiler.
for (const corpus of corpora) {
  for (const engine of ["git", "cas"]) {
    for (const id of ["000", "010", "020", "030", "099"]) {
      const file = resolve(
        output,
        "restored",
        `${corpus.name}-${engine}-${id}.typsto`,
      );
      run(cli, ["--json", "--lang", "en", "check", file]);
      const destination = resolve(
        output,
        "exports",
        `${corpus.name}-${engine}-${id}`,
      );
      if (!existsSync(destination))
        run(cli, [
          "--json",
          "--lang",
          "en",
          "export",
          file,
          "--format",
          "typst",
          "--output",
          destination,
        ]);
      // render.compile is the production compiler with only embedded dependencies.
      run(cli, ["--json", "--lang", "en", "query", "render.compile", file]);
    }
  }
}
writeFileSync(
  resolve(output, "core-validation.json"),
  JSON.stringify(
    {
      projects_checked: 30,
      typst_exports: 30,
      offline_compilations: 30,
      passed: true,
    },
    null,
    2,
  ) + "\n",
);
process.stderr.write(`Storage and core checks complete: ${output}\n`);
