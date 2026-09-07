// Simulated native events test the UI contract; real ASR is checked separately.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 940, height: 760 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => {
      let id = 0, generation = 0;
      const callbacks = new Map(), events = new Map();
      window.isTauri = true;
      window.testEvent = (event, gen = generation) => callbacks.get(events.get('recognition'))({ payload: { generation: gen, event } });
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
      window.__TAURI_INTERNALS__ = {
        transformCallback(callback) { callbacks.set(++id, callback); return id; },
        async invoke(command, args) {
          if (command === 'plugin:event|listen') { events.set(args.event, args.handler); return args.handler; }
          if (command === 'model_ready') return true;
          if (command === 'start_recording') { generation++; setTimeout(() => window.testEvent({ type: 'listening' }), 20); return generation; }
          if (command === 'stop_recording') {
            window.testEvent({ type: 'final', text: 'Keep the thought moving.' });
            window.testEvent({ type: 'stopped' }); return;
          }
          if (command === 'cancel_recording') { generation++; window.testEvent({ type: 'stopped' }); return; }
          if (command === 'plugin:clipboard-manager|write_text') { window.copiedText = args.text; return; }
          throw new Error(`Unexpected command: ${command}`);
        }
      };
    });
    await page.goto(process.env.LOGIA_UI_URL || 'http://127.0.0.1:1420');
    await page.getByRole('button', { name: 'Start recording', exact: true }).click();
    await page.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    await page.evaluate(() => window.testEvent({ type: 'partial', text: 'Keep the thought' }));
    await page.waitForFunction(() => document.querySelector('textarea').value === 'Keep the thought');
    assert(await page.getByRole('button', { name: 'Copy partial' }).isDisabled());
    await page.screenshot({ path: process.env.LOGIA_UI_SCREENSHOT || '/tmp/logia-preview-speaking.png', animations: 'disabled' });
    await page.getByRole('button', { name: 'Stop recording', exact: true }).click();
    await page.getByRole('button', { name: 'Copy text' }).click();
    assert.equal(await page.evaluate(() => window.copiedText), 'Keep the thought moving.');
    await page.getByRole('textbox', { name: 'Transcript' }).fill('Edited final text.');
    await page.getByRole('button', { name: 'Copy text' }).click();
    assert.equal(await page.evaluate(() => window.copiedText), 'Edited final text.');
    await page.getByRole('button', { name: 'Start recording', exact: true }).click();
    await page.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    await page.evaluate(() => window.testEvent({ type: 'partial', text: 'Discard this' }));
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.getByRole('button', { name: 'Start recording', exact: true }).waitFor();
    await page.evaluate(() => window.testEvent({ type: 'final', text: 'Stale text must not return' }, 2));
    assert.equal(await page.getByRole('textbox', { name: 'Transcript' }).inputValue(), '');
    await page.getByRole('button', { name: 'Dark', exact: true }).click();
    assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
    await page.setViewportSize({ width: 600, height: 600 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    assert.deepEqual(errors, []);
    console.log('Passed: live partials, Stop/final, edit/copy, Cancel, stale events, theme, narrow layout.');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
