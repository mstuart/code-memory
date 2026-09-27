const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const { execFileSync } = require("node:child_process");

const { extractTarGz } = require("./install");

test("extractTarGz treats destination paths as literal arguments", (t) => {
  const root = fs.mkdtempSync(
    path.join(os.tmpdir(), "code-memory-install-test-")
  );
  t.after(() => fs.rmSync(root, { force: true, recursive: true }));

  const source = path.join(root, "source");
  const destination = path.join(root, "$(touch injected)");
  const archive = path.join(root, "fixture.tar.gz");
  fs.mkdirSync(source);
  fs.mkdirSync(destination);
  fs.writeFileSync(path.join(source, "code-memory"), "fixture");
  execFileSync("tar", ["czf", archive, "-C", source, "code-memory"]);

  extractTarGz(fs.readFileSync(archive), destination);

  assert.equal(
    fs.readFileSync(path.join(destination, "code-memory"), "utf8"),
    "fixture"
  );
  assert.equal(
    fs.existsSync(path.join(process.cwd(), "injected")),
    false,
    "archive extraction must not execute shell syntax from paths"
  );
});
