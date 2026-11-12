// Rendering D2 source to SVG with the pinned D2 build.
//
// D2 runs from `@terrastruct/d2`, its WebAssembly build on npm, pinned to an
// exact version in package.json, so `npm ci` is the whole installation and
// local runs and CI render with byte-identical code. The SVG is normalised
// (no trailing whitespace, one final newline) so the pre-commit hooks that
// fix whitespace never make a fresh render look stale.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

export const D2_PACKAGE = "@terrastruct/d2";

const here = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = join(here, "..", "..");

/** The exact version package.json pins, and the version actually installed. */
export function d2Versions() {
  const pkg = JSON.parse(readFileSync(join(REPO_ROOT, "package.json"), "utf8"));
  const pinned = pkg.devDependencies?.[D2_PACKAGE] ?? null;
  // The package's exports map hides its package.json from `require.resolve`,
  // so read it where npm puts it.
  let installed = null;
  try {
    installed = JSON.parse(readFileSync(join(REPO_ROOT, "node_modules", D2_PACKAGE, "package.json"), "utf8")).version;
  } catch {
    installed = null;
  }
  return { pinned, installed };
}

export const INSTALL_HINT = `The D2 renderer (${D2_PACKAGE}) is not installed. Run \`npm ci\` in the repository root; it installs the exact version package.json pins, and nothing else is needed (no binary, no Docker, no network at render time).`;

/** Strips trailing whitespace from every line and ends the text with exactly one newline. */
export function normaliseSvg(svg) {
  return `${svg
    .split("\n")
    .map((l) => l.replace(/[ \t\r]+$/, ""))
    .join("\n")
    .replace(/\n+$/, "")}\n`;
}

export class Renderer {
  /** Loads D2; throws with installation instructions if it isn't there. */
  static async create() {
    const { pinned, installed } = d2Versions();
    if (pinned === null || !/^\d+\.\d+\.\d+$/.test(pinned)) {
      throw new Error(`package.json must pin ${D2_PACKAGE} to an exact version (found ${JSON.stringify(pinned)})`);
    }
    if (installed === null) throw new Error(INSTALL_HINT);
    if (installed !== pinned) {
      throw new Error(`${D2_PACKAGE} ${installed} is installed but package.json pins ${pinned}: run \`npm ci\``);
    }
    let mod;
    try {
      mod = await import(D2_PACKAGE);
    } catch (e) {
      throw new Error(`${INSTALL_HINT} (${e.message})`);
    }
    const d2 = new mod.D2();
    await d2.ready;
    return new Renderer(d2, installed);
  }

  constructor(d2, version) {
    this.d2 = d2;
    this.packageVersion = version;
  }

  /** The D2 engine's own version string, as the SVG's `data-d2-version` reports it. */
  async engineVersion() {
    return this.d2.version();
  }

  /**
   * Renders one D2 source to normalised SVG. The layout engine and theme come
   * from the source's `d2-config`. `scale: 1` makes D2 write the diagram's
   * natural `width` and `height` on the outer `<svg>`: without them a browser
   * has only the viewBox's aspect ratio, and GitHub's file viewer shrinks a
   * tall diagram into a thumbnail-sized box.
   */
  async render(source) {
    const compiled = await this.d2.compile(source, { scale: 1 });
    const svg = await this.d2.render(compiled.diagram, compiled.renderOptions);
    if (!svg.startsWith("<?xml") && !svg.startsWith("<svg")) throw new Error("D2 returned something that is not an SVG document");
    if (!/^<\?xml[^>]*\?><svg [^>]*\bwidth="\d+" height="\d+"/.test(svg)) throw new Error("D2 did not write the SVG's natural width and height; the diagram would render tiny on GitHub");
    return normaliseSvg(svg);
  }

  /** Stops the worker thread so the process can exit. */
  close() {
    this.d2.worker?.terminate();
  }
}
