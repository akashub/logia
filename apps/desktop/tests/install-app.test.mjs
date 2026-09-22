import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, access, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { replaceBundle } from '../scripts/install-app.mjs';

test('repeated installs replace the executable and remove nested app debris', async () => {
  const root = await mkdtemp(join(tmpdir(), 'logia-install-test-'));
  try {
    const source = join(root, 'build', 'Logia.app'), destination = join(root, 'Applications', 'Logia.app');
    const binary = 'Contents/MacOS/logia-desktop';
    await mkdir(join(source, 'Contents/MacOS'), { recursive: true });
    await writeFile(join(source, binary), 'current executable');
    await mkdir(join(destination, 'Logia.app/Contents/MacOS'), { recursive: true });
    await mkdir(join(destination, 'Contents/MacOS'), { recursive: true });
    await writeFile(join(destination, binary), 'stale executable');
    const verify = async path => { assert.equal(await readFile(join(path, binary), 'utf8'), 'current executable'); };
    const backup = await replaceBundle(source, destination, verify);
    assert.equal(await readFile(join(destination, binary), 'utf8'), 'current executable');
    await assert.rejects(access(join(destination, 'Logia.app')));
    assert.equal(await readFile(join(backup, binary), 'utf8'), 'stale executable');
    await replaceBundle(source, destination, verify);
    await assert.rejects(access(join(destination, 'Logia.app')));
    const rejectInstalled = async path => { if (path === destination) throw Error('verification failed'); };
    await assert.rejects(replaceBundle(source, destination, rejectInstalled), /verification failed/);
    assert.equal(await readFile(join(destination, binary), 'utf8'), 'current executable', 'previous app restored on failed installed verification');
  } finally { await rm(root, { recursive: true, force: true }); }
});
