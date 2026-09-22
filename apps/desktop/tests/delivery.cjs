const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openFixture } = require('./native-fixture.cjs');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    for (const status of ['dispatched', 'clipboard-changed', 'copied', 'copy-required', 'copy']) {
      const { main, overlay, state, emit, recognition } = await openFixture(browser);
      try {
        await main.getByRole('heading', { name: 'General', exact: true }).waitFor();
        await recognition({ type: 'stopped' }); await emit('dictation-shortcut');
        await overlay.waitForFunction(() => document.querySelector('.overlay-card')?.dataset.phase === 'recording');
        await emit('dictation-shortcut');
        await recognition({ type: 'final', text: 'The complete paragraph.' });
        await emit('delivery', { generation: state.generation, status });
        await recognition({ type: 'stopped' });
        if (status === 'dispatched') {
          await overlay.getByText('Paste sent', { exact: true }).waitFor({ timeout: 2000 });
          await overlay.waitForFunction(() => document.querySelector('.overlay-card')?.dataset.rung === 'idle');
          assert.equal(state.copied, undefined, 'frontend must not overwrite the clipboard after native paste dispatch');
        } else if (status === 'copied') {
          await overlay.getByRole('button', { name: 'Copied', exact: true }).waitFor();
          assert.equal(state.copied, undefined, 'native fallback already staged text; no asynchronous clipboard write');
        } else if (status === 'copy-required') {
          await overlay.getByText(/could not prepare this text/i).waitFor();
          assert.equal(state.copied, undefined, 'failed staging must not blindly retry');
        } else if (status === 'clipboard-changed') {
          await overlay.getByText(/clipboard changed/i).waitFor();
          assert.equal(state.copied, undefined, 'newer user clipboard is preserved');
        } else {
          await overlay.getByRole('button', { name: 'Copied', exact: true }).waitFor();
          assert.equal(state.copied, 'The complete paragraph.', 'fallback copies without a button click');
          assert.equal(await overlay.getByText(/does not expose its text field/).count(), 0, 'do not invent a reason for fallback');
        }
        if (status !== 'copy') {
          await emit('overlay-action', { action: 'edit', session: 1, text: 'Edited paragraph.' });
          await main.waitForTimeout(100);
          assert.equal(state.copied, undefined, 'editing must never trigger automatic copy');
        }
      } finally { await main.context().close(); }
    }
    console.log('Passed: automatic paste lifecycle, clipboard-change protection, automatic copy fallback.');
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
