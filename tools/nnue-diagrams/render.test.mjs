import assert from "node:assert/strict";
import { test } from "node:test";

import { Renderer, normaliseSvg } from "./render.mjs";

test("normalising strips trailing whitespace and ends with one newline", () => {
  assert.equal(normaliseSvg("<svg> \n  <g>\t\n</svg>\n\n"), "<svg>\n  <g>\n</svg>\n");
});

test("a rendered SVG carries its natural width and height, so GitHub shows it full size", { timeout: 60_000 }, async () => {
  const renderer = await Renderer.create();
  try {
    const svg = await renderer.render("direction: down\nvars: { d2-config: { layout-engine: elk } }\na -> b\n");
    const outer = /^<\?xml[^>]*\?><svg ([^>]*)>/.exec(svg)[1];
    assert.match(outer, /\bwidth="\d+"/);
    assert.match(outer, /\bheight="\d+"/);
    assert.match(outer, /\bviewBox="0 0 (\d+) (\d+)"/);
    const [, w, h] = /viewBox="0 0 (\d+) (\d+)"/.exec(outer);
    assert.match(outer, new RegExp(`width="${w}" height="${h}"`));
    assert.equal(svg, normaliseSvg(svg));
  } finally {
    renderer.close();
  }
});
