// The bot's chat greetings (lichess/config.yml) must fit Lichess's 140-character
// chat limit once lichess/run.sh has put the engine's version in: lichess-bot
// drops a longer message without sending it, so an over-long greeting would
// fail silently. Checked with a 9-character version (`v10.10.10`), longer than
// any release so far.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const MAX_CHAT_MESSAGE_LEN = 140; // lichess-bot's lib/lichess.py
const LONG_VERSION = "v10.10.10";

const config = readFileSync(new URL("../../lichess/config.yml", import.meta.url), "utf8");
const greetings = Object.fromEntries(
  [...config.matchAll(/^ {2}(hello|goodbye|hello_spectators|goodbye_spectators): "(.*)"\s*$/gm)].map(
    ([, key, text]) => [key, text],
  ),
);

test("all four greetings are present", () => {
  assert.deepEqual(Object.keys(greetings).sort(), ["goodbye", "goodbye_spectators", "hello", "hello_spectators"]);
});

test("the hellos carry the version token, filled in by run.sh", () => {
  assert.match(greetings.hello, /__VERSION__/);
  assert.match(greetings.hello_spectators, /__VERSION__/);
});

test("every greeting fits a chat message with a long version filled in", () => {
  for (const [key, text] of Object.entries(greetings)) {
    const sent = text.replaceAll("__VERSION__", LONG_VERSION);
    assert.ok(
      sent.length <= MAX_CHAT_MESSAGE_LEN,
      `${key} is ${sent.length} characters with version ${LONG_VERSION}; the limit is ${MAX_CHAT_MESSAGE_LEN}`,
    );
  }
});

test("the hello names the engine, its implementation and its data", () => {
  for (const text of [greetings.hello, greetings.hello_spectators]) {
    for (const needle of ["Braveheart", "Rust", "NNUE", "self-play"]) {
      assert.ok(text.includes(needle), `missing "${needle}" in: ${text}`);
    }
  }
});
