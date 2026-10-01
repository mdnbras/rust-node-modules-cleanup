import { build } from 'esbuild';
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const upstream = resolve(process.argv[2] ?? join(here, '.cache/upstream'));
const expected = '74e099ee05a57c683d9e7a1887b806f9f7a08d86';
const actual = execFileSync('git', ['-C', upstream, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (actual !== expected) throw new Error(`Expected upstream ${expected}, got ${actual}`);
if (execFileSync('git', ['-C', upstream, 'status', '--porcelain', '--untracked-files=no'], { encoding: 'utf8' }).trim()) {
  throw new Error('Upstream contains tracked modifications; use a clean checkout');
}
mkdirSync(join(here, '.cache'), { recursive: true });
await build({
  entryPoints: [join(here, 'js-adapter.ts')],
  outfile: join(here, '.cache/js-benchmark.mjs'),
  alias: { upstream: join(upstream, 'src') },
  nodePaths: [join(here, 'node_modules')],
  bundle: true,
  platform: 'node',
  format: 'esm',
  target: 'node22',
});
writeFileSync(join(here, '.cache/upstream.json'), JSON.stringify({ repository: 'sebastianekstrom/node-modules-cleanup', commit: actual, bundler: 'esbuild 0.25.10' }, null, 2) + '\n');
console.log(`Prepared unchanged upstream core at ${actual}`);
