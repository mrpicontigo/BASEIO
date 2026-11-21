// Vitest loads two module instances (and fails with "failed to find the
// current suite") when the cwd has a lowercase Windows drive letter, as VS Code
// terminals often produce. Normalise the drive letter, then run vitest.
import { spawnSync } from "node:child_process";
const cwd = process.cwd().replace(/^[a-z]:/, (d) => d.toUpperCase());
const r = spawnSync("npx", ["vitest", "run", ...process.argv.slice(2)], { cwd, stdio: "inherit", shell: true });
process.exit(r.status ?? 1);
