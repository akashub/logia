// Real controllers in two webviews. macOS authorization is explicitly simulated.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openFixture } = require('./native-fixture.cjs');
const until = async (page, predicate) => {
  for (let i = 0; i < 60; i++) { if (predicate()) return; await page.waitForTimeout(50); }
  assert(predicate(), 'expected controller transition');
};
(async () => {
  const browser = await chromium.launch({ headless: true });
  async function scenario(name, options, check) {
    const f = await openFixture(browser, options);
    try { await check(f); console.log(`Passed: ${name}`); } finally { await f.context.close(); }
  }
  try {
    await scenario('one action per step; automatic grant and revocation refresh', { microphone: 'denied', accessibility: false }, async ({ main, state, emit, recognition }) => {
      const panel = main.locator('.settings-permission-wizard');
      await main.getByRole('heading', { name: 'Set up Logia', exact: true }).waitFor();
      assert.equal(await panel.getByRole('button').count(), 1, 'denied microphone has one recovery action');
      assert.equal(state.micRequests || 0, 0); assert.equal(state.typingRequests || 0, 0);
      await main.getByRole('button', { name: 'Open Microphone settings' }).click();
      await until(main, () => state.openedPermission === 'microphone');
      await recognition({ type: 'stopped' }); await emit('dictation-shortcut');
      await main.waitForTimeout(100); assert.equal(state.starts, 0);
      state.microphone = 'authorized';
      await main.evaluate(() => window.dispatchEvent(new Event('focus')));
      await main.getByRole('heading', { name: 'Let Logia type for you' }).waitFor();
      assert.equal(await panel.getByRole('button').count(), 1, 'typing permission has one action');
      await main.getByRole('button', { name: 'Enable typing access', exact: true }).click();
      await until(main, () => state.typingRequests === 1);
      await until(main, () => state.openedPermission === 'accessibility');
      state.accessibility = true; await emit('permissions-refresh');
      await main.getByRole('button', { name: 'Finish setup' }).click();
      await main.getByRole('heading', { name: 'General', exact: true }).waitFor();
      state.accessibility = false; await emit('permissions-refresh');
      await main.getByRole('heading', { name: 'Let Logia type for you' }).waitFor();
      await emit('dictation-shortcut'); await main.waitForTimeout(100);
      assert.equal(state.starts, 0, 'revoked typing access returns to setup before any recording starts');
      assert.equal(await main.getByRole('button', { name: 'Finish setup' }).count(), 0);
      await main.getByRole('button', { name: 'Enable typing access', exact: true }).click();
      await until(main, () => state.typingRequests === 2);
      await until(main, () => state.openedPermission === 'accessibility');
      await main.getByRole('button', { name: 'Enable typing access', exact: true }).waitFor();
      for (const theme of ['dark', 'light']) {
        await main.evaluate(t => { document.documentElement.dataset.theme = t; }, theme);
        await main.screenshot({ path: `/tmp/logia-permissions-${theme}.png` });
      }
      await main.setViewportSize({ width: 600, height: 600 });
      assert.equal(await main.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      await main.screenshot({ path: '/tmp/logia-permissions-narrow.png' });
    });
    await scenario('microphone denial and errors have visible outcomes', { microphone: 'not_determined', accessibility: true }, async ({ main, state }) => {
      const allow = main.getByRole('button', { name: 'Allow microphone', exact: true });
      await allow.waitFor(); state.micRequestError = true;
      await allow.click(); await main.getByRole('alert').filter({ hasText: 'Microphone request failed' }).waitFor();
      state.micRequestError = false; state.micRequestResult = 'denied';
      await allow.click();
      await main.getByRole('button', { name: 'Open Microphone settings', exact: true }).waitFor();
      await until(main, () => state.openedPermission === 'microphone');
      state.settingsError = true;
      await main.getByRole('button', { name: 'Open Microphone settings', exact: true }).click();
      await main.getByRole('alert').filter({ hasText: 'Could not open System Settings' }).waitFor();
    });
    await scenario('stop shortcut remains usable after microphone revocation', {}, async ({ main, state, emit, recognition }) => {
      await main.getByRole('heading', { name: 'General', exact: true }).waitFor();
      await recognition({ type: 'stopped' }); await emit('dictation-shortcut');
      await until(main, () => state.starts === 1);
      await recognition({ type: 'listening' });
      state.microphone = 'denied'; await emit('permissions-refresh');
      await emit('dictation-shortcut'); await until(main, () => state.stops === 1);
      assert.equal(state.starts, 1, 'permission changes never start another worker');
    });
    await scenario('overlapping focus checks cannot cancel a valid global start', {}, async ({ main, state, emit, recognition }) => {
      await main.getByRole('heading', { name: 'General', exact: true }).waitFor();
      await recognition({ type: 'stopped' }); state.delayMicStatus = true;
      await emit('dictation-shortcut'); await until(main, () => state.micWaiters?.length > 0);
      await emit('permissions-refresh'); await main.waitForTimeout(50);
      state.delayMicStatus = false; state.micWaiters.forEach(resolve => resolve());
      await until(main, () => state.starts === 1);
      assert.equal(state.hidden, true, 'global start must not activate settings');
    });
    await scenario('failed permission lookup is visible and never reuses cached authorization', {}, async ({ main, state, emit, recognition }) => {
      await main.getByRole('heading', { name: 'General', exact: true }).waitFor();
      await recognition({ type: 'stopped' }); state.micStatusError = true;
      await emit('dictation-shortcut');
      await main.getByRole('alert').filter({ hasText: 'Permission status unavailable' }).waitFor();
      assert.equal(state.starts, 0);
      await main.getByRole('button', { name: 'Check permissions', exact: true }).click();
      await main.getByRole('alert').filter({ hasText: 'Permission status unavailable' }).waitFor();
    });
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
