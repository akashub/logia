// Window geometry and ASR are simulated here; this exercises the real UI controller.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 560, height: 520 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => {
      let id = 0, generation = 0;
      const callbacks = new Map(), events = new Map();
      window.isTauri = true;
      window.starts = 0; window.stops = 0; window.conflict = true;
      window.emitTest = (name, payload) => callbacks.get(events.get(name))?.({ payload });
      window.recognition = event => window.emitTest('recognition', { generation, event });
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
      window.__TAURI_INTERNALS__ = {
        transformCallback(callback) { callbacks.set(++id, callback); return id; },
        async invoke(command, args) {
          if (command === 'plugin:event|listen') {
            if (location.search.includes('fail-listener') && args.event === 'dictation-shortcut') throw 'Listener unavailable';
            events.set(args.event, args.handler); return args.handler;
          }
          if (command === 'plugin:event|unlisten') return;
          if (command === 'register_shortcut') { if (window.conflict) throw 'Shortcut is already in use. Free it in the other app, then retry.'; return '⌘ ⇧ Space'; }
          if (command === 'model_ready') return true;
          if (command === 'show_main_window') { window.hidden = false; return; }
          if (command === 'hide_main_window') {
            if (window.delayHide) await new Promise(resolve => { window.finishHide = resolve; });
            window.hidden = true; return;
          }
          if (command === 'warmup_recognizer') return generation;
          if (command === 'set_floating') {
            if (window.failWindow) throw 'Could not position the dictation window.';
            if (window.delayWindow) await new Promise(resolve => { window.finishWindow = resolve; });
            window.floating = args.floating; return;
          }
          if (command === 'start_recording') { window.starts++; generation++; setTimeout(() => window.recognition({ type: 'listening' }), 30); return generation; }
          if (command === 'stop_recording') { window.stops++; window.recognition({ type: 'final', text: 'A whole thought, with room to breathe.' }); window.recognition({ type: 'stopped' }); return; }
          if (command === 'cancel_recording') { generation++; window.recognition({ type: 'stopped' }); return; }
          if (command === 'plugin:clipboard-manager|write_text') { window.copied = args.text; return; }
          throw new Error(`Unexpected command: ${command}`);
        }
      };
    });
    await page.goto(process.env.LOGIA_UI_URL || 'http://127.0.0.1:1420');
    await page.getByRole('button', { name: 'Retry shortcut' }).waitFor({ timeout: 5000 });
    await page.evaluate(() => window.emitTest('dictation-shortcut', null));
    assert.equal(await page.evaluate(() => window.starts), 0, 'shortcut must not record during warmup');
    await page.evaluate(() => { window.conflict = false; });
    await page.getByRole('button', { name: 'Retry shortcut' }).click();
    await page.getByText('⌘ ⇧ Space', { exact: true }).waitFor();
    await page.evaluate(() => window.recognition({ type: 'stopped' }));
    await page.getByRole('textbox', { name: 'Transcript' }).fill('Keep this until a recording actually starts.');
    await page.evaluate(() => { window.failWindow = true; window.emitTest('dictation-shortcut', null); });
    await page.getByText('Could not position the dictation window.').waitFor();
    assert.equal(await page.evaluate(() => window.starts), 0);
    assert.equal(await page.locator('textarea').inputValue(), 'Keep this until a recording actually starts.');
    await page.evaluate(() => {
      window.failWindow = false; window.delayWindow = true;
      window.emitTest('dictation-shortcut', null);
      window.emitTest('dictation-shortcut', null);
    });
    await page.waitForFunction(() => Boolean(window.finishWindow));
    assert.equal(await page.evaluate(() => window.starts), 0, 'do not open mic before window is ready');
    await page.evaluate(() => { window.delayWindow = false; window.finishWindow(); });
    await page.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    assert.equal(await page.evaluate(() => window.starts), 1, 'rapid triggers must not start duplicate workers');
    assert.equal(await page.locator('main').getAttribute('data-view'), 'floating');
    assert.equal(await page.evaluate(() => window.scrollY), 0, 'floating mode must not inherit outer page scroll');
    await page.evaluate(() => window.recognition({ type: 'partial', text: 'A whole thought, with room to breathe.' }));
    await page.waitForFunction(() => document.querySelector('textarea').value.endsWith('breathe.'));
    await page.evaluate(() => window.emitTest('dictation-dismiss', null));
    await page.getByText('Stop or cancel recording before dismissing Logia.').waitFor({ timeout: 2000 });
    assert.notEqual(await page.evaluate(() => window.hidden), true, 'Close must not hide active capture');
    const box = await page.locator('textarea').boundingBox();
    await page.screenshot({ path: '/tmp/logia-floating-speaking.png', animations: 'disabled' });
    await page.evaluate(() => window.emitTest('dictation-shortcut', null));
    await page.getByRole('button', { name: 'Copy text' }).click();
    assert.equal(await page.evaluate(() => window.copied), 'A whole thought, with room to breathe.');
    assert.deepEqual(await page.locator('textarea').boundingBox(), box, 'Stop must not collapse floating paragraph');
    assert.equal(await page.locator('main').getAttribute('data-view'), 'floating');
    await page.evaluate(() => window.emitTest('dictation-dismiss', null));
    await page.waitForFunction(() => window.hidden === true);
    assert.equal(await page.locator('textarea').inputValue(), 'A whole thought, with room to breathe.', 'dismissal retains text for recovery');
    await page.getByRole('button', { name: 'Expand window', exact: true }).click();
    assert.equal(await page.locator('textarea').inputValue(), 'A whole thought, with room to breathe.');
    await page.getByRole('button', { name: 'Float window', exact: true }).click();
    // A newer Start/Cancel invalidates an older asynchronous shortcut intent.
    await page.evaluate(() => { window.delayWindow = true; window.finishWindow = null; window.emitTest('dictation-shortcut', null); });
    await page.waitForFunction(() => Boolean(window.finishWindow));
    await page.getByRole('button', { name: 'Start recording', exact: true }).click();
    await page.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.getByRole('button', { name: 'Start recording', exact: true }).waitFor();
    await page.evaluate(() => { window.delayWindow = false; window.finishWindow(); });
    await page.waitForTimeout(100);
    assert.equal(await page.evaluate(() => window.starts), 2, 'old shortcut must not restart microphone after a newer Cancel');
    await page.evaluate(() => { window.delayHide = true; window.emitTest('dictation-dismiss', null); });
    await page.waitForFunction(() => Boolean(window.finishHide));
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', metaKey: true })));
    await page.waitForTimeout(50);
    assert.equal(await page.evaluate(() => window.starts), 2, 'dismissing cannot race with a new microphone start');
    await page.evaluate(() => { window.delayHide = false; window.finishHide(); });
    await page.getByRole('button', { name: 'Start recording', exact: true }).waitFor();
    await page.setViewportSize({ width: 480, height: 460 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    assert((await page.getByRole('button', { name: 'Start recording', exact: true }).boundingBox()).y < 420, 'primary action remains visible at minimum floating size');
    const shortcutBox = await page.locator('.shortcut-note').boundingBox();
    assert(shortcutBox.y + shortcutBox.height <= 460, 'shortcut stays visible without scrolling at minimum size');
    await page.goto((process.env.LOGIA_UI_URL || 'http://127.0.0.1:1420') + '?fail-listener');
    await page.waitForFunction(() => window.hidden === false);
    await page.getByText('Could not connect the shortcut. Quit and reopen Logia.').waitFor();
    assert.equal(await page.getByRole('button', { name: 'Retry shortcut' }).count(), 0, 'registration-only retry cannot repair a missing listener');
    assert.deepEqual(errors, []);
    console.log('Passed: shortcut conflict/retry, warmup gate, window failure, rapid gestures, persistent floating paragraph, expand/copy, minimum layout.');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
