import test from 'node:test';
import assert from 'node:assert/strict';
import {surfaceRequire} from './surface-test-loader.mjs';
const {NativeMessageProfile} = surfaceRequire('./native-message-profile');

test('native message timings aggregate by type without retaining messages', () => {
    const profile = new NativeMessageProfile();
    profile.record('TextUpdate', 3);
    profile.record('ScrollerUpdate', 1);
    profile.record('TextUpdate', 2);
    const report = JSON.parse(JSON.stringify(profile.snapshot()));
    assert.deepEqual(report, [
        {type:'TextUpdate', count:2, totalMs:5, maxMs:3},
        {type:'ScrollerUpdate', count:1, totalMs:1, maxMs:1},
    ]);
    profile.record('TextUpdate', 4);
    assert.equal(report[0].count, 2, 'reports are snapshots');
});
