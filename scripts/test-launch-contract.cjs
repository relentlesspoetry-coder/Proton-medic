// Guard the Rust IPC status contract against frontend type drift.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const ts = require('typescript');
const root = path.resolve(__dirname, '..');

test('LaunchResult accepts exactly the statuses emitted by native assessment', () => {
  const source = fs.readFileSync(path.join(root, 'src/types.ts'), 'utf8');
  const ast = ts.createSourceFile('types.ts', source, ts.ScriptTarget.Latest, true);
  const result = ast.statements.find(n => ts.isInterfaceDeclaration(n) && n.name.text === 'LaunchResult');
  assert.ok(result, 'LaunchResult interface must exist');
  const status = result.members.find(n => n.name?.getText(ast) === 'status');
  assert.ok(status && ts.isUnionTypeNode(status.type), 'status must be a literal union');
  const frontend = status.type.types.map(n => {
    assert.ok(ts.isLiteralTypeNode(n) && ts.isStringLiteral(n.literal));
    return n.literal.text;
  });
  const rust = fs.readFileSync(path.join(root, 'src-tauri/src/launch_test.rs'), 'utf8');
  const assessment = rust.match(/fn assess\([\s\S]*?(?=\nfn launch_and_observe)/)?.[0];
  assert.ok(assessment, 'native assessment function must be found');
  const backend = [...assessment.matchAll(/\("([a-z-]+)",/g)].map(m => m[1]);
  assert.ok(backend.length > 0, 'native status values must be found');
  assert.deepEqual([...new Set(frontend)].sort(), [...new Set(backend)].sort());
});
