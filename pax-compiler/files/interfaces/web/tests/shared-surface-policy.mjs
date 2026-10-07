import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const root = fileURLToPath(new URL('../../../../../', import.meta.url));
// Compile once per test process; Cargo preserves the shared incremental cache.
// Production policy is never reimplemented by the fake DOM harness.
execFileSync('cargo', ['build', '--quiet', '-p', 'pax-chassis-web', '--example',
    'surface_schedule_fixture', '--features', 'designtime'],
    {cwd: root, env: {...process.env, CARGO_NET_OFFLINE: 'true'}, stdio: 'pipe'});
const executable = `${root}target/debug/examples/surface_schedule_fixture`;
export function sharedSurfaceDecisions(snapshots) {
    return JSON.parse(execFileSync(executable, {input: JSON.stringify(snapshots), encoding: 'utf8'}));
}
export const sharedSurfaceDecision = snapshot => sharedSurfaceDecisions([snapshot])[0];
