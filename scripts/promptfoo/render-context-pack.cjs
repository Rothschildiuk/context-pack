const { spawnSync } = require('node:child_process');
const path = require('node:path');

module.exports = async function renderContextPack({ vars }) {
  const repoRoot = path.resolve(__dirname, '..', '..');
  const fixtureRoot = resolveFixtureRoot(repoRoot, vars);
  const args = parseArgs(vars.args);

  const result = runContextPack(repoRoot, fixtureRoot, args);
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(
      `context-pack failed with exit code ${result.status}: ${String(result.stderr || '').trim()}`,
    );
  }

  return String(result.stdout || '').trimEnd();
};

function resolveFixtureRoot(repoRoot, vars) {
  if (vars.fixturePath) {
    return path.resolve(repoRoot, vars.fixturePath);
  }

  if (!vars.fixture) {
    throw new Error('promptfoo test is missing vars.fixture');
  }

  return path.join(repoRoot, 'tests', 'fixtures', vars.fixture);
}

function parseArgs(rawArgs) {
  if (!rawArgs) {
    return [];
  }

  if (Array.isArray(rawArgs)) {
    return rawArgs.map(String);
  }

  if (typeof rawArgs !== 'string') {
    throw new Error(`unsupported args value: ${typeof rawArgs}`);
  }

  const trimmed = rawArgs.trim();
  if (!trimmed) {
    return [];
  }

  if (trimmed.startsWith('[')) {
    const parsed = JSON.parse(trimmed);
    if (!Array.isArray(parsed)) {
      throw new Error('JSON args must decode to an array');
    }
    return parsed.map(String);
  }

  return trimmed.split(/\s+/).filter(Boolean);
}

function runContextPack(repoRoot, fixtureRoot, args) {
  const configuredBinary = process.env.CONTEXT_PACK_BIN;
  if (configuredBinary) {
    return spawnSync(
      configuredBinary,
      ['--cwd', fixtureRoot, ...args],
      {
        cwd: repoRoot,
        encoding: 'utf8',
      },
    );
  }

  return spawnSync(
    'cargo',
    ['run', '--quiet', '--', '--cwd', fixtureRoot, ...args],
    {
      cwd: repoRoot,
      encoding: 'utf8',
    },
  );
}
