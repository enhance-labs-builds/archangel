// Resolves imports with the TypeScript compiler, for the differential test in
// crates/archangel_graph/tests/resolve_differential.rs.
//
// Usage: node scripts/resolve-with-typescript.mjs <fixture folder> '<JSON list of [file, specifier]>'
// Prints a JSON list with, for each request, the resolved file relative to the fixture folder, or
// null when TypeScript cannot resolve it.
import path from 'node:path';
import ts from 'typescript';

const [fixture, requestsJson] = process.argv.slice(2);
const requests = JSON.parse(requestsJson);
const root = ts.sys.realpath(path.resolve(fixture));

const optionsFor = new Map();
function compilerOptions(file) {
  const configPath = ts.findConfigFile(path.dirname(file), ts.sys.fileExists, 'tsconfig.json');
  if (configPath === undefined) return {};
  if (!optionsFor.has(configPath)) {
    const parsed = ts.getParsedCommandLineOfConfigFile(configPath, {}, {
      ...ts.sys,
      onUnRecoverableConfigFileDiagnostic: (diagnostic) => {
        throw new Error(ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n'));
      },
    });
    optionsFor.set(configPath, parsed.options);
  }
  return optionsFor.get(configPath);
}

const results = requests.map(([file, specifier]) => {
  const absolute = path.join(root, file);
  const { resolvedModule } = ts.resolveModuleName(specifier, absolute, compilerOptions(absolute), ts.sys);
  if (resolvedModule === undefined) return null;
  const resolved = ts.sys.realpath(resolvedModule.resolvedFileName);
  return path.relative(root, resolved).split(path.sep).join('/');
});
process.stdout.write(JSON.stringify(results));
