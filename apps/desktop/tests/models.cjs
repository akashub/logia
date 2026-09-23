const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openFixture } = require('./native-fixture.cjs');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const { main, state, emit, recognition } = await openFixture(browser);
    await main.getByRole('heading', { name: 'General', exact: true }).waitFor();
    await recognition({ type: 'stopped' });
    await main.getByRole('button', { name: 'Models', exact: true }).click();
    const row = name => main.locator('.settings-model-row').filter({ has: main.getByRole('heading', { name, exact: true }) });
    const small = row('Moonshine Streaming Small'), medium = row('Moonshine Streaming Medium');
    await small.getByText('Active', { exact: true }).waitFor({ timeout: 2500 });
    assert(await small.getByRole('button', { name: 'Remove', exact: true }).isDisabled());
    const warmups = state.warmups;
    state.delayDownload = true;
    await medium.getByRole('button', { name: 'Download', exact: true }).click();
    await emit('model-progress', 37);
    await medium.getByRole('progressbar').waitFor();
    await main.waitForFunction(() => document.querySelector('progress')?.value === 37);
    assert.equal(await medium.getByRole('progressbar').getAttribute('value'), '37');
    assert(await small.getByRole('button', { name: 'Prepare again' }).isDisabled());
    await main.getByRole('button', { name: 'General', exact: true }).click();
    assert.equal(await main.getByRole('heading', { name: 'Moonshine Streaming Medium', exact: true }).count(), 0);
    await main.getByRole('button', { name: 'Models', exact: true }).click();
    await medium.getByRole('progressbar').waitFor({ timeout: 2000 });
    assert.equal(await medium.getByRole('progressbar').getAttribute('value'), '37');
    assert(await small.getByRole('button', { name: 'Prepare again' }).isDisabled());
    state.delayDownload = false; state.finishDownload();
    await medium.getByRole('button', { name: 'Use model', exact: true }).waitFor();
    assert.equal(state.warmups, warmups, 'download does not change or reload the active model');
    state.selectionError = true;
    await medium.getByRole('button', { name: 'Use model', exact: true }).click();
    await main.getByRole('alert').filter({ hasText: 'verification failed' }).waitFor();
    await small.getByText('Active', { exact: true }).waitFor();
    state.selectionError = false;
    await medium.getByRole('button', { name: 'Use model', exact: true }).click();
    await medium.getByText('Active', { exact: true }).waitFor();
    assert.equal(state.warmups, warmups + 1);
    assert(await small.getByRole('button', { name: 'Remove', exact: true }).isDisabled(), 'warmup owns the worker');
    await recognition({ type: 'stopped' });
    await small.getByRole('button', { name: 'Remove', exact: true }).click();
    await small.getByRole('button', { name: 'Download', exact: true }).waitFor();
    await medium.getByText('Active', { exact: true }).waitFor();
    assert(await medium.getByRole('button', { name: 'Remove', exact: true }).isDisabled());
    for (const theme of ['dark', 'light']) {
      await main.evaluate(theme => { document.documentElement.dataset.theme = theme; }, theme);
      await main.screenshot({ path: `/tmp/logia-model-choice-${theme}.png`, fullPage: true });
    }
    console.log('Passed: explicit model choice, active removal protection, visible download progress, failed selection retention, warmup gates and both-theme renders.');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
