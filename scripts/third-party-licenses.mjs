// Writes src/lib/settings/third-party-licenses.json, the list shown in
// Settings > About > Third-party licenses (spec 6.10). Run it after changing
// dependencies: `npm run licenses`.
//
// - Rust crates: the ones built into the app (normal dependencies of
//   parlo-player, followed through, for Windows and Linux), from `cargo metadata`.
// - Interface packages: the ones bundled into the page (BUNDLED below), the
//   Inter font included.
// - Native components: written by hand (NATIVE below), because no package
//   manager describes them. The libmpv entry reflects the build in
//   src-tauri/lib/mpv (see docs/libmpv-windows.md, section 5).
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "src", "lib", "settings", "third-party-licenses.json");

/** npm packages whose code ends up in the app's page. */
const BUNDLED = [
  "@tauri-apps/api",
  "svelte",
  "@sveltejs/kit",
  "@lucide/svelte",
  "@fontsource-variable/inter",
  "@inlang/paraglide-js",
];

const NATIVE = [
  {
    name: "libmpv (mpv)",
    version: "0.41 (shinchiro build)",
    license: "GPL-2.0-or-later",
    url: "https://github.com/mpv-player/mpv",
    note: "Built with GPL components (no -Dgpl=false).",
  },
  {
    name: "FFmpeg (inside libmpv)",
    version: "development build",
    license: "GPL-3.0-or-later",
    url: "https://ffmpeg.org/legal.html",
    note: "As configured by the shinchiro build.",
  },
];

/** The platforms the app ships for (spec 12.7). */
const TARGETS = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"];

function rustCrates() {
  const packages = new Map();
  for (const target of TARGETS) {
    const text = execFileSync(
      "cargo",
      [
        "metadata",
        "--format-version",
        "1",
        "--filter-platform",
        target,
        "--manifest-path",
        join(root, "src-tauri", "Cargo.toml"),
      ],
      { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
    );
    const metadata = JSON.parse(text);
    const byId = new Map(metadata.packages.map((p) => [p.id, p]));
    const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));

    // Normal dependencies only (kind null): build scripts and tests are not shipped.
    const seen = new Set();
    const queue = [metadata.resolve.root];
    while (queue.length > 0) {
      const id = queue.pop();
      if (seen.has(id)) continue;
      seen.add(id);
      for (const dep of nodes.get(id)?.deps ?? []) {
        if (dep.dep_kinds.some((kind) => kind.kind === null)) queue.push(dep.pkg);
      }
    }
    seen.delete(metadata.resolve.root);
    for (const id of seen) packages.set(id, byId.get(id));
  }

  return [...packages.values()]
    .map((p) => ({
      name: p.name,
      version: p.version,
      license: p.license ?? (p.license_file ? "See license file" : "Unknown"),
      url: p.repository ?? `https://crates.io/crates/${p.name}`,
    }));
}

function webPackages() {
  return BUNDLED.map((name) => {
    const manifest = JSON.parse(readFileSync(join(root, "node_modules", name, "package.json"), "utf8"));
    const repository = typeof manifest.repository === "string" ? manifest.repository : manifest.repository?.url;
    return {
      name,
      version: manifest.version,
      license: manifest.license ?? "Unknown",
      url: manifest.homepage ?? repository ?? `https://www.npmjs.com/package/${name}`,
    };
  });
}

const byName = (a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version);
const list = {
  native: NATIVE,
  web: webPackages().sort(byName),
  rust: rustCrates().sort(byName),
};
writeFileSync(output, `${JSON.stringify(list, null, 2)}\n`);
console.log(`${list.native.length} native, ${list.web.length} web, ${list.rust.length} Rust → ${output}`);
