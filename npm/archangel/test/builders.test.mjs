// A config may destructure the rule builders, as the docs do, so the types declare them as
// function properties. A method there makes typescript-eslint's unbound-method report each one.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import ts from 'typescript';

const path = new URL('../index.d.ts', import.meta.url);
const source = ts.createSourceFile('index.d.ts', readFileSync(path, 'utf8'), ts.ScriptTarget.Latest);

test('each rule builder is a function property, not a method', () => {
  const builders = source.statements.find(
    (statement) => ts.isInterfaceDeclaration(statement) && statement.name.text === 'RuleBuilders',
  );
  assert.ok(builders, 'index.d.ts declares RuleBuilders');
  const members = builders.members.map((member) => [member.name.getText(source), ts.SyntaxKind[member.kind]]);
  assert.deepEqual(
    members,
    [
      ['layers', 'PropertySignature'],
      ['isolate', 'PropertySignature'],
      ['entry', 'PropertySignature'],
      ['mayImport', 'PropertySignature'],
      ['rule', 'PropertySignature'],
    ],
    `a method in RuleBuilders is unbound when a config destructures it: ${JSON.stringify(members)}`,
  );
});
