import { afterAll, afterEach, beforeAll, expect, it } from "vitest";
import {
  spawn,
  spawnSync,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { createInterface } from "node:readline";
import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir, platform, arch, cpus } from "node:os";
import { resolve } from "node:path";
import { performance } from "node:perf_hooks";
import * as Y from "yjs";
import { getSchema, type Editor } from "@tiptap/core";
import {
  prosemirrorJSONToYXmlFragment,
  yXmlFragmentToProsemirrorJSON,
  yUndoPluginKey,
} from "@tiptap/y-tiptap";
import { createRichZone, richZoneExtensions } from "../../app/src/rich-zone";
import { initializeSettings, settingsFor } from "../../app/src/state";
import type { Provider } from "../../app/src/provider";

const root = resolve("experiments/yrs-evaluation");
const input = JSON.parse(readFileSync(`${root}/.generated/input.json`, "utf8"));
const schema = getSchema(richZoneExtensions("document"));
const docs: Y.Doc[] = [],
  editors: Editor[] = [];
const cases: { name: string; passed: boolean; detail?: unknown }[] = [];
const benchmarks: unknown[] = [];
let catalogue: any, base: Y.Doc, rust: Rpc, native: Rpc;
class Rpc {
  child: ChildProcessWithoutNullStreams;
  queue: {
    resolve: (value: any) => void;
    reject: (reason: unknown) => void;
  }[] = [];
  constructor(binary: string, args: string[] = []) {
    this.child = spawn(binary, args, { stdio: ["pipe", "pipe", "pipe"] });
    this.child.stderr.on("data", () => {});
    createInterface({ input: this.child.stdout }).on("line", (line) => {
      const next = this.queue.shift();
      if (!next) return;
      try {
        const value = JSON.parse(line);
        value.ok === false || value.error
          ? next.reject(value.error)
          : next.resolve(value.result ?? value);
      } catch (error) {
        next.reject(error);
      }
    });
    this.child.on("error", (error) =>
      this.queue.splice(0).forEach((q) => q.reject(error)),
    );
    this.child.on("exit", (code) =>
      this.queue.splice(0).forEach((q) => q.reject(`child exited ${code}`)),
    );
  }
  call(value: unknown): Promise<any> {
    return new Promise((resolve, reject) => {
      this.queue.push({ resolve, reject });
      this.child.stdin.write(JSON.stringify(value) + "\n");
    });
  }
  stop() {
    this.child.stdin.end();
  }
}
const encode = (doc: Y.Doc) =>
  Buffer.from(Y.encodeStateAsUpdate(doc)).toString("base64");
function load(encoded: string) {
  const doc = new Y.Doc();
  docs.push(doc);
  Y.applyUpdate(doc, Buffer.from(encoded, "base64"), "bootstrap");
  return doc;
}
function copy(doc = base) {
  return load(encode(doc));
}
const roots = (doc: Y.Doc) =>
  [...doc.share.keys()].filter((key) => key !== "default");
async function probe(doc: Y.Doc, steps: any[] = [], options: any = {}) {
  const project_schema =
    doc.share.has("settings") && doc.getMap("settings").has("document")
      ? catalogue.types.find(
          (t: any) =>
            t.id ===
            (doc.getMap("settings").get("document") as any).metadata.type,
        )
      : undefined;
  return rust.call({
    update: encode(doc),
    map_roots: roots(doc),
    steps,
    project_schema,
    ...options,
  });
}
function raw(value: any): any {
  if (value instanceof Y.XmlText)
    return { kind: "xml-text", delta: value.toDelta() };
  if (value instanceof Y.Text) return { kind: "text", delta: value.toDelta() };
  if (value instanceof Y.XmlElement)
    return {
      kind: "xml-element",
      name: value.nodeName,
      attrs: value.getAttributes(),
      children: value.toArray().map(raw),
    };
  if (value instanceof Y.XmlFragment)
    return { kind: "xml-fragment", children: value.toArray().map(raw) };
  if (value instanceof Y.Map)
    return {
      kind: "map",
      entries: Object.fromEntries([...value].map(([k, v]) => [k, raw(v)])),
    };
  if (value instanceof Y.Array)
    return { kind: "array", values: value.toArray().map(raw) };
  return { kind: "json", value };
}
function tree(doc: Y.Doc) {
  return Object.fromEntries(
    [...doc.share.keys()].map((name) => [
      name,
      raw(name === "default" ? doc.getXmlFragment(name) : doc.getMap(name)),
    ]),
  );
}
function at(doc: Y.Doc, path: (string | number)[]) {
  let value: any =
    path[0] === "default"
      ? doc.getXmlFragment("default")
      : doc.getMap(String(path[0]));
  for (const key of path.slice(1)) value = value.get(key);
  return value;
}
function body(doc: Y.Doc): any {
  return yXmlFragmentToProsemirrorJSON(doc.getXmlFragment("default"));
}
function metadata(doc: Y.Doc): any {
  initializeSettings({ doc } as Provider, catalogue);
  return settingsFor({ doc } as Provider).metadata;
}
function canonical(value: any): any {
  if (Array.isArray(value)) return value.map(canonical);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value)
      .filter(
        ([k, v]) =>
          !(v === null && k === "attrs") &&
          !(Array.isArray(v) && !v.length) &&
          !(k === "attrs" && v && !Object.keys(v).length),
      )
      .map(([k, v]) => [
        k,
        k === "marks" && Array.isArray(v)
          ? v.map(canonical).sort((a, b) => a.type.localeCompare(b.type))
          : canonical(v),
      ]),
  );
}
const text = (s: string, marks?: any[]) => ({
  type: "text",
  text: s,
  ...(marks ? { marks } : {}),
});
const paragraph = (s: string) => ({ type: "paragraph", content: [text(s)] });
function seeded(document: any) {
  const doc = new Y.Doc();
  docs.push(doc);
  prosemirrorJSONToYXmlFragment(
    schema,
    document,
    doc.getXmlFragment("default"),
  );
  return doc;
}
function vector(doc: Y.Doc) {
  return [...Y.decodeStateVector(Y.encodeStateVector(doc))].sort(
    ([a], [b]) => a - b,
  );
}
function checked(name: string, run: () => Promise<unknown>) {
  it(name, async () => {
    try {
      const detail = await run();
      cases.push({ name, passed: true, ...(detail ? { detail } : {}) });
    } catch (error) {
      cases.push({ name, passed: false, detail: String(error) });
      throw error;
    }
  });
}
beforeAll(async () => {
  rust = new Rpc(`${root}/target/release/typstographe-yrs-evaluation`);
  native = new Rpc(resolve("target/debug/typstographe"), [
    "--stdio",
    "--session-dir",
    mkdtempSync(`${tmpdir()}/tog-yrs-native-`),
  ]);
  catalogue = await native.call({
    peer: "yrs-evaluation",
    command: "content_catalogue",
    args: {},
  });
  if (input.state) base = load(input.state);
  else {
    base = seeded(input.document);
    initializeSettings({ doc: base } as Provider, catalogue, {
      metadata: input.metadata,
      template: input.template,
      modelId: input.manifest.model_id,
    });
  }
  metadata(base);
});
afterEach(() => {
  editors.splice(0).forEach((e) => e.destroy());
  document.body.replaceChildren();
});
afterAll(() => {
  writeFileSync(
    `${root}/results.json`,
    JSON.stringify(
      {
        versions: {
          yrs: "0.28.0",
          yrs_features: ["small-client"],
          offset_kind: "Utf16",
          skip_gc: true,
          yjs: JSON.parse(readFileSync("node_modules/yjs/package.json", "utf8"))
            .version,
          y_tiptap: JSON.parse(
            readFileSync("node_modules/@tiptap/y-tiptap/package.json", "utf8"),
          ).version,
        },
        environment: {
          platform: platform(),
          arch: arch(),
          cpu: cpus()[0]?.model,
          node: process.version,
        },
        fixture: {
          sha256: input.sha256,
          archive_bytes: input.archive_bytes,
          saved_yjs_bytes: Buffer.from(input.state, "base64").length,
          evaluated_yjs_bytes: base
            ? Buffer.from(encode(base), "base64").length
            : 0,
        },
        cases,
        benchmarks,
      },
      null,
      2,
    ) + "\n",
  );
  rust?.stop();
  native?.stop();
  docs.splice(0).forEach((d) => d.destroy());
});

checked(
  "01 — archive réelle : aller-retour complet sans édition, types partagés et identités conservés",
  async () => {
    // ProseMirror materializes default attributes (e.g. null table colwidth)
    // which are omitted in the Yjs XML representation, before Yrs is involved.
    expect(canonical(schema.nodeFromJSON(body(base)).toJSON())).toEqual(
      canonical(schema.nodeFromJSON(input.document).toJSON()),
    );
    expect(metadata(base)).toEqual(input.metadata);
    const result = await probe(base);
    expect(
      canonical(schema.nodeFromJSON(result.projected.document).toJSON()),
    ).toEqual(canonical(schema.nodeFromJSON(input.document).toJSON()));
    expect(canonical(result.projected.metadata)).toEqual(
      canonical(metadata(base)),
    );
    expect(result.tree).toEqual(tree(base));
    const restored = load(result.full);
    expect(tree(restored)).toEqual(tree(base));
    expect(vector(restored)).toEqual(vector(base));
    const originalText = at(base, ["default", 0, 0]) as Y.XmlText;
    const cursor = Y.createRelativePositionFromTypeIndex(originalText, 3);
    expect(
      Y.createAbsolutePositionFromRelativePosition(cursor, restored)?.index,
    ).toBe(3);
    expect(
      Y.createAbsolutePositionFromRelativePosition(cursor, restored)?.type._item
        ?.id,
    ).toEqual(originalText._item?.id);
    return {
      full_bytes: result.full_bytes,
      original_clients: vector(base).length,
      roots: [...base.share.keys()],
    };
  },
);
checked(
  "02 — Unicode : UTF-16 explicite, emoji, accents combinants et japonais",
  async () => {
    const doc = seeded({ type: "doc", content: [paragraph("A😀é日本Z é 👩🏽‍🔬")] });
    const step = {
      id: "text.insert",
      path: ["default", 0, 0],
      index: 5,
      text: "X",
    };
    const result = await probe(doc, [step]);
    const good = load(result.full);
    expect(at(good, ["default", 0, 0]).toString()).toBe("A😀é日X本Z é 👩🏽‍🔬");
    const negative = load(
      (await probe(doc, [step], { offset_kind: "bytes" })).full,
    );
    expect(at(negative, ["default", 0, 0]).toString()).toBe("A😀Xé日本Z é 👩🏽‍🔬");
    return { default_byte_offsets_are_not_yjs_offsets: true };
  },
);
checked(
  "03 — texte riche : gras, italique et liens préservés et modifiés depuis Rust",
  async () => {
    const doc = seeded({
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [
            text("Un texte "),
            text("gras", [{ type: "bold" }]),
            text(" et "),
            text("un lien", [
              { type: "italic" },
              {
                type: "link",
                attrs: { href: "https://example.org/recherche", title: "Lien" },
              },
            ]),
          ],
        },
      ],
    });
    const result = await probe(doc, [
      {
        id: "text.format",
        path: ["default", 0, 0],
        index: 0,
        length: 2,
        attrs: { bold: {}, italic: {} },
      },
    ]);
    const restored = load(result.full);
    expect(result.tree).toEqual(tree(restored));
    const pm = schema.nodeFromJSON(body(restored));
    pm.check();
    expect(pm.firstChild?.firstChild?.marks.map((m) => m.type.name)).toEqual([
      "bold",
      "italic",
    ]);
    expect(pm.textContent).toBe("Un texte gras et un lien");
    expect(
      pm.lastChild?.lastChild?.marks.find((m) => m.type.name === "link")?.attrs
        .href,
    ).toBe("https://example.org/recherche");
  },
);
checked(
  "04 — en-tête réel : auteur Y.Text, titre et résumé XmlFragment, ordre et sources bibliographiques",
  async () => {
    const author = input.metadata.fields.authors[0].id;
    const locale = input.metadata.fields.localizations[0].id;
    const before = metadata(base);
    const result = await probe(base, [
      {
        id: "text.insert",
        path: ["settings", "collections", "authors", author, "name"],
        index: 0,
        text: "Dr ",
      },
      {
        id: "text.insert",
        path: [
          "settings",
          "collections",
          "localizations",
          locale,
          "title",
          0,
          0,
        ],
        index: 0,
        text: "Évaluer — ",
      },
      {
        id: "text.insert",
        path: [
          "settings",
          "collections",
          "localizations",
          locale,
          "abstract",
          0,
          0,
        ],
        index: 0,
        text: "にほん 👩🏽‍🔬. ",
      },
      {
        id: "array.delete",
        path: ["settings", "order:authors"],
        index: 0,
        length: 1,
      },
      {
        id: "array.insert",
        path: ["settings", "order:authors"],
        index: before.fields.authors.length - 1,
        value: author,
      },
      {
        id: "map.set",
        path: ["settings"],
        key: "article.show_outline",
        value: !before.fields.show_outline,
      },
    ]);
    const restored = load(result.full);
    expect(result.tree).toEqual(tree(restored));
    const after = metadata(restored);
    expect(after.fields.authors.at(-1).id).toBe(author);
    expect(after.fields.authors.at(-1).name).toBe(
      "Dr " + before.fields.authors[0].name,
    );
    expect(
      after.fields.localizations[0].title.content[0].content[0].text,
    ).toContain("Évaluer — ");
    expect(
      after.fields.localizations[0].abstract.content[0].content[0].text,
    ).toContain("にほん 👩🏽‍🔬. ");
    expect(after.fields.references).toEqual(before.fields.references);
    expect(after.fields.show_outline).toBe(!before.fields.show_outline);
  },
);
checked(
  "05 — annotations et objets : attributs JSON des citations, notes, images et descriptions",
  async () => {
    const citation = {
      type: "citation",
      attrs: {
        items: [
          { file_id: "demo-books", key: "lefevre2021", supplement: "p. 18–21" },
        ],
        form: "normal",
      },
    };
    const doc = seeded({
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [
            text("Avant"),
            citation,
            { type: "footnote", attrs: { text: "Une note é😀" } },
            text("Après"),
          ],
        },
        {
          type: "figure",
          attrs: { src: "images/demo.svg", alt: "Éléments japonais 日本語" },
          content: [text("Légende")],
        },
        {
          type: "descriptionList",
          content: [
            {
              type: "descriptionItem",
              content: [
                { type: "descriptionTerm", content: [text("Terme")] },
                {
                  type: "descriptionDefinition",
                  content: [text("Définition")],
                },
              ],
            },
          ],
        },
      ],
    });
    const result = await probe(doc, [
      {
        id: "xml.attr",
        path: ["default", 0, 1],
        key: "items",
        value: [
          { file_id: "demo-books", key: "moreau2023", supplement: "chap. 2" },
        ],
      },
      {
        id: "xml.attr",
        path: ["default", 0, 2],
        key: "text",
        value: "Note corrigée 👩🏽‍🔬",
      },
      {
        id: "xml.attr",
        path: ["default", 1],
        key: "alt",
        value: "Nouveau texte alternatif",
      },
      {
        id: "text.insert",
        path: ["default", 2, 0, 0, 0],
        index: 0,
        text: "Nouveau ",
      },
    ]);
    const restored = load(result.full);
    expect(result.tree).toEqual(tree(restored));
    schema.nodeFromJSON(body(restored)).check();
    expect(body(restored).content[0].content[1].attrs.items[0].key).toBe(
      "moreau2023",
    );
    expect(body(restored).content[0].content[2].attrs.text).toBe(
      "Note corrigée 👩🏽‍🔬",
    );
    expect(body(restored).content[1].attrs.alt).toBe(
      "Nouveau texte alternatif",
    );
  },
);
checked(
  "06 — tableau : ajout Rust et édition JS simultanée dans une cellule existante",
  async () => {
    const row = (s: string) => ({
      type: "tableRow",
      content: [{ type: "tableCell", content: [paragraph(s)] }],
    });
    const doc = seeded({
      type: "doc",
      content: [
        paragraph("Avant"),
        { type: "table", content: [row("A"), row("B")] },
      ],
    });
    const js = copy(doc),
      original = at(js, ["default", 1, 1, 0, 0, 0]) as Y.XmlText;
    const id = original._item?.id;
    original.insert(1, " local");
    const result = await probe(doc, [
      {
        id: "xml.insert",
        path: ["default", 1],
        index: 0,
        node: row("Nouvelle ligne"),
      },
    ]);
    const rs = load(result.full);
    Y.applyUpdate(js, Buffer.from(result.diff, "base64"));
    Y.applyUpdate(rs, Y.encodeStateAsUpdate(js));
    expect(tree(js)).toEqual(tree(rs));
    expect(at(js, ["default", 1, 2, 0, 0, 0])._item.id).toEqual(id);
    expect(at(js, ["default", 1, 2, 0, 0, 0]).toString()).toBe("B local");
    schema.nodeFromJSON(body(js)).check();
  },
);
checked(
  "07 — convergence : insertions concurrentes au même endroit et champs indépendants",
  async () => {
    const doc = seeded({ type: "doc", content: [paragraph("Début fin")] });
    doc.getMap("settings").set("plain", "base");
    const js = copy(doc);
    at(js, ["default", 0, 0]).insert(6, "JS ");
    js.getMap("settings").set("js-field", "local");
    const result = await probe(doc, [
      { id: "text.insert", path: ["default", 0, 0], index: 6, text: "Rust " },
      {
        id: "map.set",
        path: ["settings"],
        key: "rust-field",
        value: "distant",
      },
    ]);
    const rs = load(result.full);
    Y.applyUpdate(js, Buffer.from(result.diff, "base64"));
    Y.applyUpdate(rs, Y.encodeStateAsUpdate(js));
    Y.applyUpdate(js, Y.encodeStateAsUpdate(rs));
    expect(tree(js)).toEqual(tree(rs));
    const s = at(js, ["default", 0, 0]).toString();
    expect(s).toContain("JS ");
    expect(s).toContain("Rust ");
    expect(js.getMap("settings").get("js-field")).toBe("local");
    expect(js.getMap("settings").get("rust-field")).toBe("distant");
  },
);
checked(
  "08 — valeurs atomiques concurrentes : convergence déterministe, sans fusion métier",
  async () => {
    const doc = seeded({ type: "doc", content: [paragraph("Texte")] });
    doc.getMap("settings").set("associations", ["initial"]);
    const js = copy(doc);
    js.getMap("settings").set("associations", ["initial", "js"]);
    const result = await probe(doc, [
      {
        id: "map.set",
        path: ["settings"],
        key: "associations",
        value: ["initial", "rust"],
      },
    ]);
    const rs = load(result.full);
    Y.applyUpdate(js, Buffer.from(result.diff, "base64"));
    Y.applyUpdate(rs, Y.encodeStateAsUpdate(js));
    expect(tree(js)).toEqual(tree(rs));
    expect(js.getMap("settings").get("associations")).not.toEqual([
      "initial",
      "js",
      "rust",
    ]);
    return { atomic_lists_do_not_merge_members: true };
  },
);
checked(
  "09 — mises à jour différentielles : livraison inversée, répétition et dépendances en attente",
  async () => {
    const doc = seeded({ type: "doc", content: [paragraph("Texte")] });
    const one = await probe(doc, [
      {
        id: "text.insert",
        path: ["default", 0, 0],
        index: 0,
        text: "Premier ",
      },
    ]);
    const first = load(one.full);
    const two = await probe(first, [
      { id: "text.insert", path: ["default", 0, 0], index: 0, text: "Second " },
    ]);
    const target = copy(doc);
    Y.applyUpdate(target, Buffer.from(two.diff, "base64"));
    Y.applyUpdate(target, Buffer.from(two.diff, "base64"));
    Y.applyUpdate(target, Buffer.from(one.diff, "base64"));
    Y.applyUpdate(target, Buffer.from(one.diff, "base64"));
    expect(tree(target)).toEqual(tree(load(two.full)));
    const back = await probe(target);
    expect(back.tree).toEqual(tree(target));
  },
);
checked(
  "10 — positions relatives : ancrage conservé après insertion Rust et suppression voisine",
  async () => {
    const doc = seeded({ type: "doc", content: [paragraph("avant après")] });
    const item = at(doc, ["default", 0, 0]) as Y.XmlText;
    const anchor = Y.createRelativePositionFromTypeIndex(item, 6);
    const result = await probe(
      doc,
      [
        { id: "text.insert", path: ["default", 0, 0], index: 0, text: "👩🏽‍🔬 " },
        { id: "text.delete", path: ["default", 0, 0], index: 8, length: 2 },
      ],
      {
        anchor: Buffer.from(Y.encodeRelativePosition(anchor)).toString(
          "base64",
        ),
      },
    );
    const restored = load(result.full);
    const p = Y.createAbsolutePositionFromRelativePosition(anchor, restored);
    expect(p?.index).toBe(12);
    expect(p?.type._item?.id).toEqual(item._item?.id);
    expect(result.anchor.offset).toBe(12);
    expect(
      Y.createAbsolutePositionFromRelativePosition(
        Y.decodeRelativePosition(Buffer.from(result.anchor.encoded, "base64")),
        restored,
      )?.index,
    ).toBe(12);
  },
);
checked(
  "11 — annuler/rétablir humain dans le vrai éditeur sans effacer une édition Rust",
  async () => {
    const doc = seeded({
      type: "doc",
      content: [
        paragraph("Bonjour"),
        paragraph("Autre passage"),
        { type: "paragraph" },
      ],
    });
    const element = document.createElement("div");
    document.body.append(element);
    const editor = createRichZone({
      profile: "document",
      element,
      label: "Évaluation Yrs",
      language: "fr",
      binding: { kind: "document", document: doc },
    });
    editors.push(editor);
    if (!editor.isInitialized)
      await new Promise<void>((resolve) =>
        editor.once("create", () => resolve()),
      );
    editor.commands.insertContentAt(8, " humain");
    yUndoPluginKey.getState(editor.state).undoManager.stopCapturing();
    const result = await probe(doc, [
      { id: "text.insert", path: ["default", 1, 0], index: 0, text: "Rust : " },
    ]);
    Y.applyUpdate(doc, Buffer.from(result.diff, "base64"), "yrs-remote");
    expect(editor.state.doc.textContent).toContain("Bonjour humain");
    expect(editor.state.doc.textContent).toContain("Rust : Autre passage");
    expect(editor.commands.undo()).toBe(true);
    expect(editor.state.doc.textContent).not.toContain("humain");
    expect(editor.state.doc.textContent).toContain("Rust : Autre passage");
    expect(editor.commands.redo()).toBe(true);
    expect(editor.state.doc.textContent).toContain("Bonjour humain");
    expect(editor.state.doc.textContent).toContain("Rust : Autre passage");
  },
);
checked(
  "12 — limite : supprimer un bloc en concurrence avec une édition de son texte",
  async () => {
    const doc = seeded({
      type: "doc",
      content: [paragraph("À retirer"), paragraph("Conserver")],
    });
    const js = copy(doc);
    at(js, ["default", 0, 0]).insert(0, "édition distante ");
    const result = await probe(doc, [
      { id: "xml.delete", path: ["default"], index: 0, length: 1 },
    ]);
    const rs = load(result.full);
    Y.applyUpdate(js, Buffer.from(result.diff, "base64"));
    Y.applyUpdate(rs, Y.encodeStateAsUpdate(js));
    expect(tree(js)).toEqual(tree(rs));
    expect(JSON.stringify(body(js))).not.toContain("édition distante");
    return {
      deletion_hides_concurrent_text: true,
      needs_application_conflict_policy: true,
    };
  },
);
checked(
  "13 — article édité par Rust : validation et export Typst réels, état sauvegardable",
  async () => {
    const author = input.metadata.fields.authors[0].id;
    const result = await probe(base, [
      {
        id: "text.insert",
        path: ["default", 1, 0],
        index: 0,
        text: "Évaluation Yrs : ",
      },
      {
        id: "text.insert",
        path: ["settings", "collections", "authors", author, "name"],
        index: 0,
        text: "Dr ",
      },
    ]);
    const restored = load(result.full);
    expect(canonical(result.projected.document)).toEqual(
      canonical(body(restored)),
    );
    expect(canonical(result.projected.metadata)).toEqual(
      canonical(metadata(restored)),
    );
    const snapshot = {
      revision: 0,
      document: result.projected.document,
      settings: {
        metadata: result.projected.metadata,
        template: input.template,
        modelId: input.manifest.model_id,
      },
    };
    await native.call({
      peer: "yrs-evaluation",
      command: "validate_model",
      args: { snapshot },
    });
    const exported = await native.call({
      peer: "yrs-evaluation",
      command: "export_document",
      args: { snapshot },
    });
    writeFileSync(
      `${root}/.generated/edited.json`,
      JSON.stringify({
        document: snapshot.document,
        metadata: snapshot.settings.metadata,
        state: result.full,
      }),
    );
    writeFileSync(`${root}/.generated/export-path.txt`, exported);
    return { native_typst_validation: true, exported: true };
  },
);
checked(
  "14 — limite : Yrs accepte un badge dont la référence manque, le cœur le refuse",
  async () => {
    const result = await probe(base, [
      {
        id: "xml.insert",
        path: ["default", 1],
        index: 0,
        node: {
          type: "citation",
          attrs: {
            items: [{ file_id: "absent", key: "introuvable", supplement: "" }],
            form: "normal",
          },
        },
      },
    ]);
    const restored = load(result.full);
    const snapshot = {
      revision: 0,
      document: body(restored),
      settings: {
        metadata: metadata(restored),
        template: input.template,
        modelId: input.manifest.model_id,
      },
    };
    await expect(
      native.call({
        peer: "yrs-evaluation",
        command: "validate_model",
        args: { snapshot },
      }),
    ).rejects.toContain("Référence citée absente");
    return { crdt_validity_is_not_document_validity: true };
  },
);
checked(
  "15 — mesures release : article réel et deux documents plus longs",
  async () => {
    for (const [name, doc] of [
      ["article", base],
      [
        "long",
        seeded({
          type: "doc",
          content: Array.from({ length: 250 }, (_, i) =>
            paragraph(
              `${i}. ` + "Texte de recherche é😀 et 日本語. ".repeat(30),
            ),
          ),
        }),
      ],
      [
        "large",
        seeded({
          type: "doc",
          content: Array.from({ length: 1000 }, (_, i) =>
            paragraph(
              `${i}. ` + "Texte de recherche é😀 et 日本語. ".repeat(40),
            ),
          ),
        }),
      ],
    ] as const) {
      const measures: any[] = [];
      const update = encode(doc),
        map_roots = roots(doc);
      for (let i = 0; i < 24; i++) {
        const started = performance.now();
        const r = await rust.call({
          update,
          map_roots,
          tree: false,
          steps: [
            {
              id: "text.insert",
              path: ["default", 0, 0],
              index: 0,
              text: "Rust ",
            },
          ],
        });
        if (i >= 4)
          measures.push({
            ...r.timings_us,
            wall_ms: performance.now() - started,
            full_bytes: r.full_bytes,
            diff_bytes: r.diff_bytes,
          });
      }
      const summarize = (key: string) => {
        const values = measures.map((m) => m[key]).sort((a, b) => a - b);
        return {
          median: values[Math.floor(values.length / 2)],
          p95: values[Math.ceil(values.length * 0.95) - 1],
        };
      };
      const request = {
        update,
        map_roots,
        tree: false,
        steps: [
          {
            id: "text.insert",
            path: ["default", 0, 0],
            index: 0,
            text: "Rust ",
          },
        ],
      };
      writeFileSync(
        `${root}/.generated/bench-${name}.json`,
        JSON.stringify(request) + "\n",
      );
      benchmarks.push({
        name,
        iterations: measures.length,
        input_bytes: Buffer.from(update, "base64").length,
        output_bytes: measures[0].full_bytes,
        diff_bytes: measures[0].diff_bytes,
        decode_us: summarize("decode"),
        edit_us: summarize("edit"),
        encode_us: summarize("encode"),
        core_total_us: summarize("total"),
        stdio_wall_ms: summarize("wall_ms"),
      });
    }
  },
);
checked(
  "16 — annuler/rétablir dans Rust et relire le résultat dans Yjs",
  async () => {
    const doc = seeded({ type: "doc", content: [paragraph("Avant")] });
    const steps = [
      { id: "text.insert", path: ["default", 0, 0], index: 5, text: " après" },
    ];
    const undone = await probe(doc, steps, { history: true, undo: true });
    expect(undone.undo).toBe(true);
    expect(body(load(undone.full))).toEqual(body(doc));
    const redone = await probe(doc, steps, {
      history: true,
      undo: true,
      redo: true,
    });
    expect(redone.redo).toBe(true);
    expect(at(load(redone.full), ["default", 0, 0]).toString()).toBe(
      "Avant après",
    );
  },
);
checked(
  "17 — sauvegarde .typsto réelle, réouverture dans le vrai éditeur et métadonnées concordantes",
  async () => {
    const result = spawnSync(
      resolve("target/debug/examples/yrs_archive_probe"),
      [
        input.source || resolve("examples/article-working.typsto"),
        `${root}/.generated/edited.json`,
        `${root}/.generated/edited.typsto`,
        `${root}/.generated/reopened.json`,
      ],
      { encoding: "utf8" },
    );
    expect(result.status, result.stderr).toBe(0);
    const reopened = JSON.parse(
      readFileSync(`${root}/.generated/reopened.json`, "utf8"),
    );
    const doc = load(reopened.state);
    expect(canonical(schema.nodeFromJSON(body(doc)).toJSON())).toEqual(
      canonical(schema.nodeFromJSON(reopened.document).toJSON()),
    );
    expect(canonical(metadata(doc))).toEqual(canonical(reopened.metadata));
    const element = document.createElement("div");
    document.body.append(element);
    const editor = createRichZone({
      profile: "document",
      element,
      label: "Projet rouvert",
      language: "fr",
      binding: { kind: "document", document: doc },
    });
    editors.push(editor);
    if (!editor.isInitialized)
      await new Promise<void>((resolve) =>
        editor.once("create", () => resolve()),
      );
    expect(canonical(editor.getJSON())).toEqual(
      canonical(schema.nodeFromJSON(reopened.document).toJSON()),
    );
    expect(editor.state.doc.textContent).toContain("Évaluation Yrs : ");
    const countCitations = (n: any): number =>
      Number(n.type === "citation") +
      (n.content || []).reduce(
        (count: number, child: any) => count + countCitations(child),
        0,
      );
    expect(editor.view.dom.querySelectorAll(".citation-ref")).toHaveLength(
      countCitations(input.document),
    );
    expect(
      yUndoPluginKey.getState(editor.state).undoManager.undoStack,
    ).toHaveLength(0);
    return {
      compiled_pages: reopened.pages,
      yjs_and_archive_agree: true,
      undo_stack_not_persisted: true,
    };
  },
);
checked(
  "18 — trente-deux fusions successives avec Unicode et styles concurrents",
  async () => {
    let current = seeded({
      type: "doc",
      content: [paragraph("é😀 にほん é 👩🏽‍🔬 fin")],
    });
    for (let i = 0; i < 32; i++) {
      const js = copy(current),
        value = at(js, ["default", 0, 0]) as Y.XmlText;
      const result = await probe(current, [
        {
          id: "text.insert",
          path: ["default", 0, 0],
          index: 0,
          text: i % 2 ? "é" : "😀",
        },
        {
          id: "text.format",
          path: ["default", 0, 0],
          index: 0,
          length: i % 2 ? 1 : 2,
          attrs: { bold: {} },
        },
      ]);
      js.transact(() => {
        value.insert(value.length, i % 2 ? " 日本" : " é");
        value.format(0, Array.from(value.toString())[0].length, { italic: {} });
      }, "js-local");
      const rs = load(result.full);
      Y.applyUpdate(js, Buffer.from(result.diff, "base64"));
      Y.applyUpdate(rs, Y.encodeStateAsUpdate(js));
      Y.applyUpdate(js, Y.encodeStateAsUpdate(rs));
      expect(tree(js)).toEqual(tree(rs));
      schema.nodeFromJSON(body(js)).check();
      current = js;
    }
    return { cycles: 32 };
  },
);
checked(
  "19 — rétention : effet du ramasse-miettes sur les états sérialisés",
  async () => {
    const doc = seeded({
      type: "doc",
      content: [
        paragraph("Texte répété pour mesurer la rétention des suppressions."),
      ],
    });
    const s = at(doc, ["default", 0, 0]).toString();
    const steps = Array.from({ length: 500 }, () => [
      {
        id: "text.delete",
        path: ["default", 0, 0],
        index: 0,
        length: s.length,
      },
      { id: "text.insert", path: ["default", 0, 0], index: 0, text: s },
    ]).flat();
    const retained = await probe(doc, steps, { skip_gc: true });
    const collected = await probe(doc, steps, { skip_gc: false });
    expect(body(load(retained.full))).toEqual(body(doc));
    expect(body(load(collected.full))).toEqual(body(doc));
    expect(retained.full_bytes).toBeGreaterThan(collected.full_bytes);
    return {
      replacements: 500,
      retained_bytes: retained.full_bytes,
      collected_bytes: collected.full_bytes,
    };
  },
);
