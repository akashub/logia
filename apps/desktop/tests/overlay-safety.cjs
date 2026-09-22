// ASR/native calls are doubled; acknowledgments and controls run in real React pages.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openFixture } = require('./native-fixture.cjs');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const failures = [];
  async function check(name, run, options) {
    const fixture = await openFixture(browser, options);
    try {
      await fixture.main.getByRole('heading', { name: 'General', exact: true }).waitFor();
      await fixture.recognition({ type: 'stopped' });
      await run(fixture);
      console.log(`Passed: ${name}`);
    } catch (error) { failures.push(`${name}: ${error.message}`); }
    finally { await fixture.context.close(); }
  }
  try {
    await check('Voice test reveals an acknowledged Preparing panel before capture with idle disabled', async ({ main, state }) => {
      // claude 2026-09-11: the idle dot now defaults to off, so this starts
      // in the disabled state the case needs without toggling anything.
      await main.getByRole('button', { name: 'Voice test', exact: true }).click();
      await main.getByRole('button', { name: 'Start recording', exact: true }).click();
      await main.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
      assert.equal(state.visibleAtStart, true, 'voice test opened the mic without revealing the panel');
      assert.deepEqual(state.panelAtStart, { phase: 'loading', rung: 'armed' });
      await main.getByRole('button', { name: 'General', exact: true }).click();
      assert.equal(state.overlayVisible, true, 'leaving Voice test must retain visible capture controls');
    });
    await check('an unconnected panel cannot open the microphone', async ({ main, state, emit }) => {
      await emit('dictation-shortcut');
      await main.waitForTimeout(250);
      assert.equal(state.starts, 0, 'unconnected overlay must fail closed');
      await main.getByRole('alert').filter({ hasText: /overlay/i }).waitFor({ timeout: 4000 });
      assert.equal(state.hidden, false, 'failure must reveal settings');
      assert.deepEqual(state.discarded, ['1']);
    }, { connectOverlay: false });
    await check('Preparing must be acknowledged before capture', async ({ main, state, emit }) => {
      state.dropApplied = true;
      await emit('dictation-shortcut');
      await main.waitForTimeout(250);
      assert.equal(state.starts, 0, 'a successful emit is not an acknowledgment');
      await main.getByRole('alert').filter({ hasText: /overlay/i }).waitFor({ timeout: 4000 });
      assert.equal(state.hidden, false);
    });
    await check('lost bridge opens working recovery controls during capture', async ({ main, state, emit, recognition }) => {
      await emit('dictation-shortcut');
      await main.waitForTimeout(150);
      assert.equal(state.starts, 1);
      state.dropOverlayEvents = true;
      await recognition({ type: 'partial', text: 'Keep this text while recovering the controls.' });
      await main.getByRole('button', { name: 'Stop recording', exact: true }).waitFor({ timeout: 4000 });
      assert.equal(state.hidden, false);
      await main.getByRole('button', { name: 'Stop recording', exact: true }).click();
      assert.equal(state.stops, 1);
      await main.getByRole('button', { name: 'Cancel', exact: true }).click();
      await main.getByRole('button', { name: 'Cancel', exact: true }).waitFor({ state: 'hidden' });
    });
    await check('shortcut registration preserves newer preference edits', async ({ main, state }) => {
      state.delayShortcut = true;
      await main.getByRole('combobox', { name: 'Global shortcut' }).selectOption('Alt+Shift+Space');
      await main.getByRole('button', { name: 'Apply', exact: true }).click();
      await main.getByRole('combobox', { name: 'Theme', exact: true }).selectOption('dark');
      await main.getByRole('combobox', { name: 'Overlay position' }).selectOption('top');
      await main.getByRole('switch', { name: 'Show idle indicator' }).click();
      state.delayShortcut = false; state.finishShortcut();
      await main.waitForFunction(() => JSON.parse(localStorage.getItem('logia.preferences.v1')).shortcut === 'Alt+Shift+Space');
      const prefs = await main.evaluate(() => JSON.parse(localStorage.getItem('logia.preferences.v1')));
      assert.deepEqual(prefs, { shortcut: 'Alt+Shift+Space', theme: 'dark', position: 'top', showIdle: true });
    });
    assert.deepEqual(failures, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
