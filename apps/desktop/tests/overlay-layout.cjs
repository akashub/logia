// Real overlay component; native sizing and session messages are simulated.
const assert = require('node:assert/strict');

module.exports = async browser => {
  const context = await browser.newContext({ viewport: { width: 520, height: 400 }, reducedMotion: 'reduce' });
  try {
    const page = await context.newPage();
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => {
      let id = 0; const callbacks = new Map(), events = new Map();
      window.actions = []; window.isTauri = true;
      window.emitOverlay = (event, payload) => callbacks.get(events.get(event))?.({ payload });
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
      window.__TAURI_INTERNALS__ = {
        transformCallback(fn) { callbacks.set(++id, fn); return id; },
        async invoke(command, args) {
          if (command === 'plugin:event|listen') { events.set(args.event, args.handler); return args.handler; }
          if (command === 'plugin:event|unlisten') return;
          if (command === 'resize_overlay') { window.panelSize = args; return; }
          if (command === 'overlay_editing_token') return 1;
          if (command === 'set_overlay_editing') {
            if (window.failEditing && args.editing) throw Error('Native panel unavailable');
            window.editing = args.editing; return;
          }
          if (command === 'plugin:event|emit_to') {
            if (args.event === 'overlay-ready') window.overlayReady = true;
            if (args.event === 'overlay-applied') window.applied = args.payload;
            if (args.event === 'overlay-action') window.actions.push(args.payload);
            return;
          }
          throw Error(`Unexpected overlay command: ${command}`);
        }
      };
    });
    await page.goto(`${process.env.LOGIA_UI_URL || 'http://127.0.0.1:1420'}?overlay`);
    await page.waitForFunction(() => window.overlayReady);
    await page.evaluate(() => document.fonts.ready);
    let seq = 0, session = 0;
    let snapshot;
    const card = page.locator('.overlay-card'), transcript = page.locator('.overlay-transcript');
    async function send(patch) {
      snapshot = { ...snapshot, ...patch, seq: ++seq };
      await page.evaluate(value => window.emitOverlay('overlay-state', value), snapshot);
      await page.waitForFunction(expected => window.applied?.seq === expected, seq);
      await page.waitForTimeout(40);
    }
    async function screenshot(theme, state) {
      await page.screenshot({ path: `/tmp/logia-overlay-${theme}-${state}.png`, animations: 'disabled' });
    }
    async function controlsFit() {
      const overlaps = await page.locator('.overlay-row').evaluate(row => {
        const bounds = row.getBoundingClientRect();
        const children = [...row.children].filter(child => getComputedStyle(child).visibility !== 'hidden' && child.getBoundingClientRect().width > 0);
        return children.some((child, index) => {
          const box = child.getBoundingClientRect(), previous = children[index - 1]?.getBoundingClientRect();
          return box.left < bounds.left - 1 || box.right > bounds.right + 1 || previous && box.left < previous.right - 1;
        });
      });
      assert.equal(overlaps, false, 'status and controls must fit without overlap');
    }
    for (const theme of ['light', 'dark']) {
      await page.evaluate(theme => { document.documentElement.dataset.theme = theme; }, theme);
      snapshot = { seq: 0, session: ++session, phase: 'ready', text: '', complete: false, error: '',
        seconds: 0, delivery: 'copy', copied: false, dismissed: true, shortcut: '⌃ ⌥ Space', position: 'bottom', showIdle: true, level: 0, deaf: false };
      await send({}); const pip = await page.locator('.overlay-pip').elementHandle();
      await screenshot(theme, 'idle');
      assert.equal(Math.round((await card.boundingBox()).width), 14);
      await send({ phase: 'loading', dismissed: false }); await screenshot(theme, 'preparing'); await controlsFit();
      await send({ phase: 'recording', delivery: 'armed', level: 0.6 }); await screenshot(theme, 'armed'); await controlsFit();
      await send({ text: 'You', seconds: 2 }); await screenshot(theme, 'one-word');
      const short = await card.boundingBox();
      assert(short.width >= 260 && short.width <= 320, 'one word must not occupy a full-width card');
      assert.equal(Math.round(short.height), 72, 'one line uses one line of transcript height');
      assert.equal(await transcript.getAttribute('readonly'), '', 'recording text stays read-only');
      assert.equal(await page.locator('.overlay-shortcut').count(), 0, 'no shortcut hint in the recording row');
      await controlsFit();
      await send({ deaf: true, level: 0, seconds: 5 }); await screenshot(theme, 'no-input');
      await page.getByRole('status').filter({ hasText: 'No input' }).waitFor();
      const explanation = page.locator('.overlay-warning');
      assert.match(await explanation.textContent(), /input device/i);
      assert.match(await explanation.textContent(), /mute/i);
      assert.match(await explanation.textContent(), /Microphone/i);
      assert(await explanation.evaluate(node => node.scrollHeight <= node.clientHeight && node.scrollWidth <= node.clientWidth), 'full remediation must be visible');
      await controlsFit();
      const warningBox = await card.boundingBox();
      await send({ deaf: false, level: 0.7 });
      assert.deepEqual(await card.boundingBox(), warningBox, 'resumed input cannot automatically shrink the card');
      // A new recording resets the previous session's size.
      await send({ session: ++session, text: 'A short thought.', seconds: 1 });
      const paragraph = 'The whole thought stays available while the recording card remains compact. '.repeat(18);
      await send({ text: paragraph, seconds: 12 }); await screenshot(theme, 'speaking');
      const full = await card.boundingBox();
      assert.equal(Math.round(full.width), 440); assert.equal(Math.round(full.height), 96);
      assert.equal(await transcript.inputValue(), paragraph);
      await send({ level: 0, seconds: 15 }); await screenshot(theme, 'paused');
      assert.deepEqual(await card.boundingBox(), full, 'pauses must not shrink');
      await send({ text: 'A revised thought.' });
      assert.deepEqual(await card.boundingBox(), full, 'shorter revisions must not shrink');
      await send({ text: paragraph });
      await page.getByRole('button', { name: 'Expand transcript' }).click();
      assert.equal(await transcript.evaluate(node => node.clientHeight), 120); await screenshot(theme, 'expanded');
      await page.getByRole('button', { name: 'Compact transcript' }).click();
      assert.deepEqual(await card.boundingBox(), full);
      await send({ phase: 'finishing', complete: true }); await screenshot(theme, 'finishing');
      assert.deepEqual(await card.boundingBox(), full);
      await send({ phase: 'ready', delivery: 'sent' }); await screenshot(theme, 'success');
      assert(await pip.evaluate(node => node.isConnected), 'one pip survives the complete lifecycle');
      await send({ dismissed: true }); assert.equal(Math.round((await card.boundingBox()).width), 14);
      await send({ session: ++session, dismissed: false, text: 'Please keep this paragraph.', delivery: 'uncertain' });
      await screenshot(theme, 'recovery'); await controlsFit();
      assert.equal(await transcript.getAttribute('readonly'), '', 'recovery needs explicit native focus before editing');
      await transcript.click();
      await page.waitForFunction(() => window.editing === true);
      await transcript.fill('An explicit correction.');
      assert.equal(await page.evaluate(() => window.actions.at(-1).text), 'An explicit correction.');
      await send({ dismissed: true });
      await page.waitForFunction(() => window.editing === false);
      await send({ dismissed: false });
      await page.evaluate(() => { window.failEditing = true; });
      await transcript.click();
      await page.getByRole('alert').filter({ hasText: 'Open Recovery' }).waitFor();
      assert.equal(await transcript.getAttribute('readonly'), '', 'failed native focus must stay read-only');
      await page.evaluate(() => { window.failEditing = false; });
      await transcript.click();
      await page.waitForFunction(() => window.editing === true);
      await page.getByRole('button', { name: 'Copy again' }).click();
      assert.equal(await page.evaluate(() => window.actions.at(-1).action), 'copy');
      await send({ phase: 'canceling', complete: false }); await screenshot(theme, 'canceling');
      await page.waitForFunction(() => window.editing === false);
      assert.equal(await page.getByRole('button', { name: 'Cancel', exact: true }).isDisabled(), true);
    }
    assert.deepEqual(errors, []);
    console.log('Passed: content-sized overlay, uncrowded controls, full silence explanation, real meter, no shrink, explicit expansion, both-theme lifecycle renders.');
  } finally { await context.close(); }
};
