const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openFixture } = require('./native-fixture.cjs');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    await require('./overlay-layout.cjs')(browser);
    const { main, overlay, state, emit, recognition } = await openFixture(browser);
    const errors = [];
    for (const page of [main, overlay]) page.on('pageerror', error => errors.push(error.message));
    // A recording-first main window is a product regression.
    await main.getByRole('heading', { name: 'General', exact: true }).waitFor({ timeout: 4000 });
    assert.equal(await main.getByRole('textbox', { name: 'Transcript' }).count(), 0);
    assert.equal(await main.getByRole('button', { name: 'Start recording', exact: true }).count(), 0);
    assert.equal(state.warmups, 1, 'second webview must not initialize another recognizer');
    await emit('dictation-shortcut'); assert.equal(state.starts, 0, 'warmup gates capture');
    await recognition({ type: 'stopped' });
    const card = overlay.locator('.overlay-card');
    await overlay.waitForFunction(() => document.querySelector('.overlay-card')?.dataset.rung === 'idle');
    await overlay.waitForTimeout(320);
    const pip = await overlay.locator('.overlay-pip').elementHandle();
    assert.equal(Math.round((await card.boundingBox()).width), 14);
    state.failWindow = true; await emit('dictation-shortcut');
    await main.getByRole('alert').filter({ hasText: 'Could not position' }).waitFor();
    assert.equal(state.starts, 0); assert.deepEqual(state.discarded, ['1']);
    state.failWindow = false; state.delayWindow = true;
    await emit('dictation-shortcut'); await emit('dictation-shortcut');
    await main.waitForTimeout(80); assert.equal(state.starts, 0);
    state.delayWindow = false; state.finishWindow();
    await overlay.getByRole('button', { name: 'Cancel', exact: true }).waitFor();
    await overlay.waitForFunction(() => document.querySelector('.overlay-card')?.dataset.phase === 'recording');
    assert.equal(state.starts, 1); assert.equal(state.target, '2'); assert.equal(state.capturesBeforeReveal, 2);
    await overlay.waitForTimeout(200);
    assert.equal(Math.round((await card.boundingBox()).width), 176);
    const paragraph = 'The complete thought remains available while the card stays small. '.repeat(25);
    await recognition({ type: 'partial', text: paragraph });
    // claude 2026-09-11: the transcript is an editable textbox now, not a
    // read-only region — editing happens here instead of a second window.
    const transcript = overlay.locator('.overlay-transcript');
    await overlay.waitForFunction(text => document.querySelector('.overlay-transcript')?.value === text, paragraph);
    await overlay.waitForTimeout(200);
    const box = await card.boundingBox();
    assert.equal(Math.round(box.width), 440); assert.equal(Math.round(box.height), 96);
    assert.equal(await transcript.evaluate(e => e.clientHeight), 48);
    await recognition({ type: 'partial', text: '' });
    await overlay.waitForTimeout(200);
    assert.equal(await transcript.textContent(), paragraph);
    assert.deepEqual(await card.boundingBox(), box, 'silence/empty interim cannot collapse');
    await transcript.evaluate(e => { e.scrollTop = 0; e.dispatchEvent(new Event('scroll')); });
    await recognition({ type: 'partial', text: paragraph + 'Another sentence.' });
    await overlay.waitForTimeout(200);
    assert.equal(await transcript.evaluate(e => e.scrollTop), 0, 'reader owns scroll position');
    await overlay.getByRole('button', { name: 'Follow latest words' }).click();
    assert(await transcript.evaluate(e => e.scrollTop > 0));
    await overlay.getByRole('button', { name: 'Expand transcript' }).click();
    await overlay.waitForTimeout(200); assert.equal(await transcript.evaluate(e => e.clientHeight), 120);
    await overlay.getByRole('button', { name: 'Compact transcript' }).click();
    await overlay.waitForTimeout(200); assert.deepEqual(await card.boundingBox(), box);
    // claude 2026-09-11: `state.hidden` starts true (the app launches as a
    // menu-bar utility), so asserting on it could never fail meaningfully.
    // Assert instead that dismissal did not *call* hide during capture.
    const hidesBefore = state.calls.filter(name => name === 'hide_main_window').length;
    await emit('dictation-dismiss');
    await overlay.waitForTimeout(120);
    assert.equal(state.calls.filter(name => name === 'hide_main_window').length, hidesBefore,
      'active Close must not hide capture');
    await overlay.getByRole('button', { name: 'Stop recording', exact: true }).click();
    await overlay.waitForTimeout(200); assert.deepEqual(await card.boundingBox(), box);
    await recognition({ type: 'final', text: paragraph });
    await emit('delivery', { generation: state.generation, status: 'sent' });
    await recognition({ type: 'stopped' });
    await overlay.getByText('Text sent', { exact: true }).waitFor();
    await overlay.waitForFunction(() => document.querySelector('.overlay-card')?.dataset.rung === 'idle');
    assert(await pip.evaluate(e => e.isConnected), 'same pip persists through success');
    await main.getByRole('button', { name: 'Recovery', exact: true }).click();
    assert.equal(await main.getByRole('textbox', { name: 'Transcript' }).inputValue(), paragraph);
    await main.getByRole('button', { name: 'Copy text', exact: true }).click(); assert.equal(state.copied, paragraph);
    await emit('dictation-dismiss'); await main.waitForTimeout(100);
    assert.equal(await main.getByRole('textbox', { name: 'Transcript' }).inputValue(), paragraph, 'dismissal retains text');
    // Stale delayed reveal cannot restart after newer test Start/Cancel.
    state.delayWindow = true; state.finishWindow = null; await emit('dictation-shortcut');
    await main.getByRole('button', { name: 'Voice test', exact: true }).click();
    await main.getByRole('button', { name: 'Start recording', exact: true }).click();
    await main.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    assert.equal(state.target, null);
    await main.getByRole('button', { name: 'Cancel', exact: true }).click();
    await main.getByRole('button', { name: 'Start recording', exact: true }).waitFor();
    state.delayWindow = false; state.finishWindow(); await main.waitForTimeout(150);
    assert.equal(state.starts, 2, 'stale shortcut cannot reopen capture');
    state.delayHide = true; await emit('dictation-dismiss'); await main.waitForTimeout(80);
    await main.getByRole('button', { name: 'Start recording', exact: true }).click({ force: true });
    assert.equal(state.starts, 2, 'hide in flight blocks start');
    state.delayHide = false; state.finishHide(); await main.waitForTimeout(80);
    await main.getByRole('button', { name: 'Start recording', exact: true }).click();
    await main.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    state.delayCancel = true;
    await main.getByRole('button', { name: 'Cancel', exact: true }).click();
    await main.getByRole('button', { name: 'General', exact: true }).click();
    await emit('dictation-dismiss');
    await state.finishCancel(); await main.waitForTimeout(100);
    await main.getByRole('button', { name: 'Recovery', exact: true }).click();
    assert.equal(await main.getByRole('textbox', { name: 'Transcript' }).inputValue(), 'The authoritative delivered paragraph.');
    assert.deepEqual(errors, []);
    console.log('Passed: settings-first, one controller, full overlay flow/size/scroll, capture ordering, rapid triggers, stale intent, hide guard, retained text, committed Cancel.');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
