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
      window.startCount = 0;
      window.testEvent = (event, gen = generation) => callbacks.get(events.get('recognition'))({ payload: { generation: gen, event } });
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
      window.__TAURI_INTERNALS__ = {
        transformCallback(callback) { callbacks.set(++id, callback); return id; },
        async invoke(command, args) {
          if (command === 'plugin:event|listen') { events.set(args.event, args.handler); return args.handler; }
          if (command === 'model_ready') return true;
          if (command === 'register_shortcut') return '⌘ ⇧ Space';
          if (command === 'set_floating') return;
          if (command === 'warmup_recognizer') { window.completePreparation = () => window.testEvent({ type: 'stopped' }); return generation; }
          if (command === 'start_recording') { window.startCount++; generation++; setTimeout(() => window.testEvent({ type: 'listening' }), 20); return generation; }
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
    await page.getByRole('button', { name: 'Preparing voice model…', exact: true }).waitFor();
    assert.equal(await page.getByRole('button', { name: 'Float window', exact: true }).count(), 1, 'native preview offers a floating window');
    assert(await page.getByRole('button', { name: 'Preparing voice model…', exact: true }).isDisabled());
    assert.equal(await page.evaluate(() => window.startCount), 0, 'preparation must never start recording');
    await page.evaluate(() => window.completePreparation());
    await page.getByRole('button', { name: 'Start recording', exact: true }).click();
    await page.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    const before = await page.getByRole('textbox', { name: 'Transcript' }).boundingBox();
    await page.evaluate(() => {
      window.renderedValues = [];
      const element = document.querySelector('textarea');
      const descriptor = Object.getOwnPropertyDescriptor(element, 'value') || Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value');
      Object.defineProperty(element, 'value', {
        configurable: true, get() { return descriptor.get.call(this); },
        set(value) { window.renderedValues.push(value); descriptor.set.call(this, value); }
      });
      window.testEvent({ type: 'partial', text: 'Keep the thought moving through a whole paragraph.' });
    });
    await page.waitForFunction(() => document.querySelector('textarea').value === 'Keep the thought moving through a whole paragraph.');
    assert(await page.evaluate(() => window.renderedValues.some(value => value.length > 0 && value.length < 'Keep the thought moving through a whole paragraph.'.length)), 'new words should appear progressively, not as one replacement');
    await page.evaluate(() => window.testEvent({ type: 'partial', text: '' }));
    await page.waitForTimeout(250); // Exceed the entire bounded presentation window.
    assert.equal(await page.getByRole('textbox', { name: 'Transcript' }).inputValue(), 'Keep the thought moving through a whole paragraph.', 'empty provisional update must not erase the paragraph');
    const paused = await page.getByRole('textbox', { name: 'Transcript' }).boundingBox();
    assert.deepEqual(paused, before, 'pauses must not resize or move the writing surface');
    assert.equal(await page.locator('.breathing').count(), 0, 'no duplicate collapsing caption bubble');
    await page.evaluate(() => window.testEvent({ type: 'partial', text: 'Keep the thought' }));
    await page.waitForFunction(() => document.querySelector('textarea').value === 'Keep the thought');
    assert(await page.getByRole('button', { name: 'Copy partial' }).isDisabled());
    await page.screenshot({ path: process.env.LOGIA_UI_SCREENSHOT || '/tmp/logia-preview-speaking.png', animations: 'disabled' });
    // A long paragraph follows incoming words until the reader scrolls away.
    const paragraph = 'We can pause, think, and keep speaking. The whole paragraph stays in one place while the next sentence arrives. '.repeat(12);
    await page.evaluate(text => window.testEvent({ type: 'partial', text }), paragraph);
    await page.waitForFunction(text => document.querySelector('textarea').value === text, paragraph);
    assert(await page.evaluate(() => { const e = document.querySelector('textarea'); return e.scrollTop > 0 && e.scrollHeight - e.clientHeight - e.scrollTop < 48; }));
    await page.getByRole('textbox', { name: 'Transcript' }).evaluate(e => { e.scrollTop = 0; e.dispatchEvent(new Event('scroll')); });
    await page.getByRole('button', { name: 'Follow latest words ↓' }).waitFor();
    await page.evaluate(text => window.testEvent({ type: 'partial', text: text + 'Another thought follows.' }), paragraph);
    await page.waitForFunction(text => document.querySelector('textarea').value === text + 'Another thought follows.', paragraph);
    assert.equal(await page.getByRole('textbox', { name: 'Transcript' }).evaluate(e => e.scrollTop), 0, 'new words must not pull the reader away from earlier text');
    await page.getByRole('button', { name: 'Follow latest words ↓' }).click();
    assert(await page.getByRole('textbox', { name: 'Transcript' }).evaluate(e => e.scrollTop > 0));
    await page.getByRole('button', { name: 'Stop recording', exact: true }).click();
    await page.getByRole('button', { name: 'Copy text' }).click();
    assert.deepEqual(await page.getByRole('textbox', { name: 'Transcript' }).boundingBox(), before, 'Stop preserves paragraph geometry');
    assert.equal(await page.evaluate(() => window.copiedText), 'Keep the thought moving.');
    await page.getByRole('textbox', { name: 'Transcript' }).fill('Edited final text.');
    await page.getByRole('button', { name: 'Copy text' }).click();
    assert.equal(await page.evaluate(() => window.copiedText), 'Edited final text.');
    await page.getByRole('button', { name: 'Start recording', exact: true }).click();
    await page.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    await page.evaluate(() => window.testEvent({ type: 'partial', text: 'Discard this long pending stream of words before its presentation finishes.' }));
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.getByRole('button', { name: 'Start recording', exact: true }).waitFor();
    await page.evaluate(() => window.testEvent({ type: 'final', text: 'Stale text must not return' }, 2));
    await page.waitForTimeout(180);
    assert.equal(await page.getByRole('textbox', { name: 'Transcript' }).inputValue(), '');
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.getByRole('button', { name: 'Start recording', exact: true }).click();
    await page.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    await page.evaluate(() => { window.renderedValues = []; window.testEvent({ type: 'partial', text: 'Reduced motion shows received text immediately.' }); });
    await page.waitForFunction(() => document.querySelector('textarea').value === 'Reduced motion shows received text immediately.');
    assert.deepEqual(await page.evaluate(() => window.renderedValues.filter(Boolean)), ['Reduced motion shows received text immediately.']);
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.getByRole('button', { name: 'Dark', exact: true }).click();
    assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
    await page.setViewportSize({ width: 600, height: 600 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    assert.deepEqual(errors, []);
    console.log('Passed: progressive words, pause/Stop layout, empty interim, scroll ownership, final/edit/copy, Cancel/stale events, reduced motion, theme and narrow layout.');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
