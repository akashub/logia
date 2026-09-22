import { cp, lstat, mkdir, mkdtemp, readFile, readdir, rename, rm } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const exists = async path => { try { await lstat(path); return true; } catch (e) { if (e.code === 'ENOENT') return false; throw e; } };
const digest = async path => createHash('sha256').update(await readFile(join(path, 'Contents/MacOS/logia-desktop'))).digest('hex');

// Copy into a NEW sibling, never into an existing .app. Verify before and after
// replacement; rollback stays outside the installed bundle and is never merged.
export async function replaceBundle(source, destination, verify) {
  await verify(source);
  const parent = dirname(destination);
  await mkdir(parent, { recursive: true });
  const staging = await mkdtemp(join(parent, '.logia-stage-'));
  const backup = join(parent, `.logia-previous-${randomUUID()}`);
  let saved = false, installed = false;
  try {
    await cp(source, staging, { recursive: true });
    await verify(staging);
    if (await exists(destination)) { await rename(destination, backup); saved = true; }
    await rename(staging, destination); installed = true;
    await verify(destination);
    return saved ? backup : null;
  } catch (e) {
    if (installed) await rm(destination, { recursive: true });
    if (saved) await rename(backup, destination);
    throw e;
  } finally { await rm(staging, { recursive: true, force: true }); }
}

async function install() {
  const desktop = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const source = join(desktop, 'src-tauri/target/debug/bundle/macos/Logia.app');
  const destination = '/Applications/Logia.app';
  const expected = await digest(source);
  const run = (bin, args) => execFileSync(bin, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  const verify = async app => {
    if ((await readdir(app, { recursive: true })).some(path => path.endsWith('.app'))) throw Error('Nested application in bundle; refusing installation.');
    const identifier = run('/usr/bin/plutil', ['-extract', 'CFBundleIdentifier', 'raw', '-o', '-', join(app, 'Contents/Info.plist')]).trim();
    if (identifier !== 'com.akashub.logia') throw Error('Unexpected app identity');
    run('/usr/bin/codesign', ['--verify', '--deep', '--strict', app]);
    if (await digest(app) !== expected) throw Error('Installed executable differs from the built executable');
  };
  await verify(source);
  const registry = '/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister';
  const unregister = app => {
    try { run(registry, ['-u', app]); }
    catch { console.warn(`Could not unregister ${app}; the installed app will be registered again.`); }
  };
  // Stop before replacing executable pages. Never run install and quit in parallel.
  let pids = [];
  try { pids = run('/usr/bin/pgrep', ['-x', 'logia-desktop']).trim().split('\n').map(Number); }
  catch (e) { if (e.status !== 1) throw e; }
  for (const pid of pids) { try { process.kill(pid, 'SIGTERM'); } catch (e) { if (e.code !== 'ESRCH') throw e; } }
  for (let i = 0; i < 50 && pids.length; i++) {
    await new Promise(resolve => setTimeout(resolve, 100));
    pids = pids.filter(pid => { try { process.kill(pid, 0); return true; } catch (e) { if (e.code === 'ESRCH') return false; throw e; } });
  }
  if (pids.length) throw Error('Logia has not quit. Installation canceled.');
  if (await exists(destination)) {
    // Unregister the old paths before moving them, including malformed nested copies.
    for (const path of await readdir(destination, { recursive: true })) {
      if (path.endsWith('.app')) unregister(join(destination, path));
    }
    unregister(destination);
  }
  const backup = await replaceBundle(source, destination, verify);
  // Rollbacks are zip archives, not additional runnable apps in Launchpad.
  const legacy = '/private/tmp/logia-build-artifacts/Logia-debug.app';
  for (const [app, label] of [[backup, 'previous'], [source, 'build'], [legacy, 'legacy-build']]) {
    if (!app || !await exists(app)) continue;
    const archive = join(tmpdir(), `logia-${label}-${randomUUID()}.zip`);
    run('/usr/bin/ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent', app, archive]);
    unregister(app);
    await rm(app, { recursive: true });
    console.log(`Preserved ${label} at ${archive}`);
  }
  run(registry, ['-f', destination]);
  run('/usr/bin/open', [destination]);
  console.log(`Installed and opened ${destination}\nVerified executable SHA-256: ${expected}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  install().catch(error => { console.error(error.stderr?.toString() || error.message); process.exitCode = 1; });
}
