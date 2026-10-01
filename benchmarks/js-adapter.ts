import { performance } from 'node:perf_hooks';
import { findNodeModulesFolders } from 'upstream/core/findNodeModulesFolders.ts';
import { calculateSizeOfNodeModulesDirs } from 'upstream/core/calculateSizeOfNodeModulesDirs.ts';
import { deleteFolders } from 'upstream/core/deleteFolders.ts';

const root = process.argv[2];
if (!root) throw new Error('Missing benchmark fixture path');
// Exclude progress rendering in both implementations. Core source remains unchanged.
const write = process.stdout.write.bind(process.stdout);
process.stdout.write = (() => true) as typeof process.stdout.write;
const started = performance.now();
const paths = await findNodeModulesFolders(root);
const { entries, totalSize } = await calculateSizeOfNodeModulesDirs({ nodeModulesDirs: paths });
const scanned = performance.now();
await deleteFolders(entries);
const deleted = performance.now();
write(JSON.stringify({
  folders: entries.length,
  bytes: totalSize,
  scan_ms: scanned - started,
  delete_ms: deleted - scanned,
  core_ms: deleted - started,
}) + '\n');
