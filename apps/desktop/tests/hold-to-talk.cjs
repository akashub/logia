const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openFixture } = require('./native-fixture.cjs');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const { main, overlay, state, emit, recognition } = await openFixture(browser);
    const until = async predicate => {
      const deadline = Date.now() + 5000;
      while (!predicate()) { assert(Date.now() < deadline, 'fixture action did not arrive'); await new Promise(r => setTimeout(r, 10)); }
    };
    await main.getByRole('heading', { name: 'General', exact: true }).waitFor();
    await recognition({ type: 'stopped' });
    const mode = main.getByRole('combobox', { name: 'Shortcut behavior' });
    await mode.waitFor({ timeout: 2500 });
    assert.equal(await mode.inputValue(), 'toggle');
    state.conflict = true;
    await mode.selectOption('hold');
    await main.getByRole('alert').filter({ hasText: 'already in use' }).waitFor();
    assert.equal(await mode.inputValue(), 'toggle');
    state.conflict = false;
    await mode.selectOption('hold');
    await until(() => state.shortcutMode === 'hold');
    const warmups = state.warmups;
    await main.reload(); await until(() => state.warmups > warmups);
    await recognition({ type: 'stopped' });
    await overlay.reload(); // A real app restart recreates both webviews.
    await main.waitForFunction(() => document.querySelector('select[aria-label="Shortcut behavior"]')?.value === 'hold');

    let id = 0;
    const press = () => { id++; return emit('dictation-shortcut', { id: String(id), mode: 'hold', phase: 'pressed' }); };
    const release = (gesture = id) => emit('dictation-shortcut', { id: String(gesture), mode: 'hold', phase: 'released' });
    // Normal held recording; repeats and unrelated release do not stop it.
    await press(); await until(() => state.starts === 1);
    await overlay.locator('[data-phase="recording"]').waitFor();
    await main.waitForTimeout(250); // Let the intentional overlay size transition settle for its screenshot.
    const fs = require('node:fs/promises');
    await fs.mkdir('../../review/hold-to-talk', { recursive: true });
    await overlay.screenshot({ path: '../../review/hold-to-talk/armed.png' });
    await emit('dictation-shortcut', { id: String(id), mode: 'hold', phase: 'pressed' });
    await release(999); assert.equal(state.stops ?? 0, 0);
    await release(); await until(() => state.stops === 1);
    await recognition({ type: 'listening' }); // Late start event must not undo Finishing.
    await overlay.locator('[data-phase="finishing"]').waitFor();
    await release(); assert.equal(state.stops, 1);
    await recognition({ type: 'stopped' });

    // Release during permission lookup must never capture a target or open input.
    state.delayMicStatus = true;
    await press(); await until(() => state.micWaiters?.length);
    await release(); state.delayMicStatus = false;
    state.micWaiters.splice(0).forEach(resolve => resolve());
    await main.waitForTimeout(80); assert.equal(state.starts, 1);
    // Repeat at each async stage before the worker is requested.
    for (const [delay, completion] of [['delayCapture', 'finishCapture'], ['delayWindow', 'finishWindow']]) {
      state[delay] = true; state[completion] = undefined;
      await press(); await until(() => state[completion]);
      await release(); state[delay] = false; state[completion]();
      await main.waitForTimeout(80); assert.equal(state.starts, 1, delay);
    }
    state.dropApplied = true;
    await press(); await overlay.locator('[data-phase="loading"]').waitFor();
    await release(); state.dropApplied = false;
    await emit('overlay-applied', { seq: state.lastOverlayState.seq, session: state.lastOverlayState.session });
    await overlay.locator('[data-phase="ready"]').waitFor();
    assert.equal(state.starts, 1);

    // Release while start IPC is unresolved: finish after launch; never lose release.
    state.delayStart = true; state.suppressListening = true;
    await press(); await until(() => state.finishStart);
    await release(); state.delayStart = false; state.finishStart();
    await until(() => state.stops === 2);
    await recognition({ type: 'stopped' }); state.suppressListening = false;
    // Cancel invalidates ownership. A stale release cannot stop a newer hold.
    await press(); await overlay.locator('[data-phase="recording"]').waitFor();
    const canceled = id;
    await overlay.getByRole('button', { name: 'Cancel', exact: true }).click();
    await overlay.locator('[data-phase="ready"]').waitFor();
    await press(); await overlay.locator('[data-phase="recording"]').waitFor();
    await release(canceled); assert.equal(state.stops, 2);
    await recognition({ type: 'partial', text: 'A held shortcut shows live words while speaking.' });
    await main.waitForTimeout(250);
    await overlay.screenshot({ path: '../../review/hold-to-talk/speaking.png' });
    await overlay.getByRole('button', { name: 'Stop recording', exact: true }).click();
    await until(() => state.stops === 3);
    await release(); assert.equal(state.stops, 3);
    await recognition({ type: 'stopped' });
    // Deliberate Voice test belongs to its own controls, not an unrelated release.
    await main.getByRole('button', { name: 'Voice test', exact: true }).click();
    await main.getByRole('button', { name: 'Start recording', exact: true }).click();
    await overlay.locator('[data-phase="recording"]').waitFor();
    await press(); await release(); assert.equal(state.stops, 3);
    await overlay.getByRole('button', { name: 'Cancel', exact: true }).click();
    await overlay.locator('[data-phase="ready"]').waitFor();
    await main.getByRole('button', { name: 'General', exact: true }).click();
    for (const theme of ['light', 'dark']) {
      await main.getByRole('combobox', { name: 'Theme', exact: true }).selectOption(theme);
      await main.screenshot({ path: `../../review/hold-to-talk/general-${theme}.png`, fullPage: true });
    }
    await mode.selectOption('toggle');
    await until(() => state.shortcutMode === 'toggle');
    await emit('dictation-shortcut'); await overlay.locator('[data-phase="recording"]').waitFor();
    await emit('dictation-shortcut'); await until(() => state.stops === 4);
    await recognition({ type: 'stopped' });
    await mode.selectOption('hold'); await until(() => state.shortcutMode === 'hold');
    for (const [delay, completion] of [['delayMicStatus', 'micWaiters'], ['delayCapture', 'finishCapture'], ['delayWindow', 'finishWindow']]) {
      const starts = state.starts, stops = state.stops;
      state[completion] = completion === 'micWaiters' ? [] : undefined;
      state[delay] = true;
      await press(); await until(() => completion === 'micWaiters' ? state.micWaiters.length : state[completion]);
      await release(); await press(); // The replacement is still held while old work completes.
      state[delay] = false;
      if (completion === 'micWaiters') state.micWaiters.splice(0).forEach(resolve => resolve());
      else state[completion]();
      await until(() => state.starts === starts + 1);
      await overlay.locator('[data-phase="recording"]').waitFor();
      await release(); await until(() => state.stops === stops + 1);
      await recognition({ type: 'stopped' });
    }
    // A replacement released before the old request resolves must stay canceled.
    const starts = state.starts;
    state.delayCapture = true; state.finishCapture = undefined;
    await press(); await until(() => state.finishCapture);
    await release(); await press(); await release();
    state.delayCapture = false; state.finishCapture();
    await main.waitForTimeout(150); assert.equal(state.starts, starts);
    // Deliberate Voice test replaces every pending shortcut intent, including
    // permission requests and queued gestures that have not reached start yet.
    for (const [delay, completion, keyboard] of [['delayMicStatus', 'micWaiters', false], ['delayCapture', 'finishCapture', true]]) {
      state[completion] = completion === 'micWaiters' ? [] : undefined;
      state[delay] = true;
      await press(); await until(() => completion === 'micWaiters' ? state.micWaiters.length : state[completion]);
      if (keyboard) { await release(); await press(); }
      await main.getByRole('button', { name: 'Voice test', exact: true }).click();
      if (keyboard) await main.keyboard.press('Control+Enter');
      else await main.getByRole('button', { name: 'Start recording', exact: true }).click();
      await overlay.locator('[data-phase="recording"]').waitFor();
      await recognition({ type: 'partial', text: 'A deliberate voice test supersedes the old shortcut.' });
      await overlay.getByRole('button', { name: 'Stop recording', exact: true }).click();
      await recognition({ type: 'stopped' });
      await overlay.locator('[data-phase="ready"]').waitFor();
      const started = state.starts;
      state[delay] = false;
      if (completion === 'micWaiters') state.micWaiters.splice(0).forEach(resolve => resolve());
      else state[completion]();
      await main.waitForTimeout(150); assert.equal(state.starts, started, 'Voice test must invalidate pending global start');
      await release();
    }
    console.log('PASS hold-to-talk: persistence, failed config, repeats, release at startup boundaries, owned stop, cancel, voice test and toggle');
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exit(1); });
