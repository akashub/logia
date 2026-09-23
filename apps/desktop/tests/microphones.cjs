const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openFixture } = require('./native-fixture.cjs');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const { main, state, recognition } = await openFixture(browser);
    async function until(predicate) {
      const deadline = Date.now() + 3000;
      while (!predicate()) {
        assert(Date.now() < deadline, 'native fixture operation did not start');
        await new Promise(resolve => setTimeout(resolve, 10));
      }
    }
    await main.getByRole('heading', { name: 'General', exact: true }).waitFor();
    await recognition({ type: 'stopped' });
    state.inputs = { devices: [{ id: 'first', name: 'USB Microphone' }, { id: 'second', name: 'USB Microphone' }], selected: null, default_id: 'first', error: null };
    const refresh = () => main.evaluate(() => window.dispatchEvent(new Event('focus')));
    const picker = main.getByRole('combobox', { name: 'Microphone', exact: true });
    await picker.waitFor({ timeout: 3000 });
    await refresh();
    await main.waitForFunction(() => document.querySelector('option[value="second"]'));
    assert.notEqual(await picker.locator('option[value="first"]').textContent(), await picker.locator('option[value="second"]').textContent(), 'identical names need distinguishable labels');
    await picker.selectOption('second');
    await main.waitForFunction(() => document.querySelector('select[aria-label="Microphone"]').value === 'second');
    assert.equal(state.inputs.selected.id, 'second');
    const warmups = state.warmups;
    await main.reload();
    await until(() => state.warmups > warmups);
    await recognition({ type: 'stopped' });
    await main.waitForFunction(() => document.querySelector('select[aria-label="Microphone"]')?.value === 'second');
    await main.waitForFunction(() => !document.querySelector('select[aria-label="Microphone"]').disabled);

    // Unplug is a visible unavailable choice, never a quiet default switch.
    state.inputs.devices = [state.inputs.devices[0]];
    await refresh();
    await picker.locator('option[value="second"]').filter({ hasText: 'unavailable' }).waitFor({ state: 'attached' });
    assert.equal(await picker.inputValue(), 'second');
    await main.getByText('Reconnect it or choose another microphone.', { exact: false }).waitFor();
    state.inputs.devices.push({ id: 'second', name: 'USB Microphone' });
    await refresh();
    await main.waitForFunction(() => !document.querySelector('option[value="second"]').textContent.includes('unavailable'));

    // An old refresh must not roll back a more recent selection, including navigation.
    state.delayInputs = true; await refresh();
    await until(() => state.finishInputs);
    state.delayInputSelection = true;
    await picker.selectOption('first');
    await until(() => state.finishInputSelection);
    assert(await picker.isDisabled());
    await main.getByRole('button', { name: 'Models', exact: true }).click();
    state.delayInputs = false; state.finishInputs();
    state.finishInputSelection(); state.delayInputSelection = false;
    await main.getByRole('button', { name: 'General', exact: true }).click();
    await main.waitForFunction(() => document.querySelector('select[aria-label="Microphone"]').value === 'first');
    state.inputSelectionError = true;
    await picker.selectOption('second');
    await main.getByRole('alert').filter({ hasText: 'Microphone is unavailable' }).waitFor();
    assert.equal(await picker.inputValue(), 'first');
    state.inputSelectionError = false;

    // Invalid saved preferences stay visibly unknown, and explicit default repairs them.
    state.inputs.error = 'Could not read your microphone choice. Choose a microphone again.';
    state.inputs.selected = null;
    await refresh();
    await main.waitForFunction(() => document.querySelector('select[aria-label="Microphone"]').selectedOptions[0].textContent === 'Choose microphone');
    await picker.selectOption('');
    await main.waitForFunction(() => document.querySelector('select[aria-label="Microphone"]').value === '');
    assert.equal(state.inputs.error, null);
    await recognition({ type: 'loading' });
    await main.waitForFunction(() => document.querySelector('select[aria-label="Microphone"]').disabled);
    await recognition({ type: 'stopped' });
    await main.waitForFunction(() => !document.querySelector('select[aria-label="Microphone"]').disabled);
    assert.equal(state.starts, 0, 'enumerating and selecting must not start recording');

    const { mkdir } = require('node:fs/promises');
    await mkdir('../../review/microphones', { recursive: true });
    for (const theme of ['light', 'dark']) {
      await main.getByRole('combobox', { name: 'Theme', exact: true }).selectOption(theme);
      await main.screenshot({ path: `../../review/microphones/general-${theme}.png`, fullPage: true });
    }
    await main.setViewportSize({ width: 650, height: 760 });
    await main.screenshot({ path: '../../review/microphones/general-narrow.png', fullPage: true });
    assert(await picker.evaluate(el => el.getBoundingClientRect().right <= innerWidth));
    console.log('PASS microphone choice: identity, persistence, unplug/reconnect, stale refresh, navigation, failure, repair, busy state, no capture');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exit(1); });
