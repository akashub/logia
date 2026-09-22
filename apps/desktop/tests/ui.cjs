// Real settings + voice-test UI, with native recognition/clipboard IPC simulated.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openFixture } = require('./native-fixture.cjs');
(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const { main: page, overlay, state, recognition, emit, url } = await openFixture(browser);
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.getByRole('heading', { name: 'General', exact: true }).waitFor();
    // claude 2026-09-10: assert the General page's own warming banner. The
    // "Preparing local recognition" note lives on the Models page, so this
    // timed out on the General page the suite actually opens.
    await page.getByText('Loading local recognition').waitFor();
    assert.equal(state.starts, 0, 'startup never records');
    await recognition({ type: 'stopped' });
    await page.screenshot({ path: '/tmp/logia-settings-general.png' });
    await page.getByRole('combobox', { name: 'Overlay position' }).selectOption('top');
    await page.waitForTimeout(80); assert.equal(state.size.position, 'top');
    await page.getByRole('switch', { name: 'Show idle indicator' }).click();
    await page.waitForTimeout(80); assert(state.calls.includes('hide_overlay'));
    await page.getByRole('combobox', { name: 'Theme', exact: true }).selectOption('dark');
    assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
    await page.getByRole('combobox', { name: 'Global shortcut' }).selectOption('Alt+Shift+Space');
    state.conflict = true;
    await page.getByRole('button', { name: 'Apply', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'Shortcut is already in use' }).waitFor();
    assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('logia.preferences.v1')).shortcut), 'Control+Alt+Space', 'failed binding is never persisted');
    state.conflict = false;
    await page.getByRole('button', { name: 'Apply', exact: true }).click();
    await page.waitForFunction(() => JSON.parse(localStorage.getItem('logia.preferences.v1')).shortcut === 'Alt+Shift+Space');
    await page.reload(); await page.getByRole('heading', { name: 'General', exact: true }).waitFor();
    assert.equal(await page.getByRole('combobox', { name: 'Overlay position' }).inputValue(), 'top');
    // claude 2026-09-11: the idle dot now defaults to off, so the single click
    // above turns it on. This still checks the choice survived a reload.
    assert.equal(await page.getByRole('switch', { name: 'Show idle indicator' }).getAttribute('aria-checked'), 'true');
    await recognition({ type: 'stopped' });
    await page.getByRole('combobox', { name: 'Theme', exact: true }).selectOption('light');
    await page.getByRole('button', { name: 'Models', exact: true }).click();
    await page.getByRole('heading', { name: 'Moonshine Streaming Small', exact: true }).waitFor();
    await page.screenshot({ path: '/tmp/logia-settings-models.png' });
    await page.getByRole('button', { name: 'Advanced', exact: true }).click();
    await page.getByText('No rewriting', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Voice test', exact: true }).click();
    assert.equal(state.starts, 0, 'opening voice test is not permission to open the microphone');
    await page.getByRole('button', { name: 'Start recording', exact: true }).click();
    await page.getByRole('button', { name: 'Stop recording', exact: true }).waitFor();
    const transcript = page.getByRole('textbox', { name: 'Transcript' });
    const box = await transcript.boundingBox();
    await page.evaluate(() => {
      window.renderedValues = [];
      const element = document.querySelector('textarea');
      const descriptor = Object.getOwnPropertyDescriptor(element, 'value') || Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value');
      Object.defineProperty(element, 'value', { configurable:true, get() { return descriptor.get.call(this); }, set(value) { window.renderedValues.push(value); descriptor.set.call(this, value); } });
    });
    const short = 'Keep the thought moving through a whole paragraph.';
    await recognition({ type: 'partial', text: short });
    await page.waitForFunction(text => document.querySelector('textarea').value === text, short);
    assert(await page.evaluate(text => window.renderedValues.some(value => value.length > 0 && value.length < text.length), short));
    await recognition({ type: 'partial', text: '' }); await page.waitForTimeout(180);
    assert.equal(await transcript.inputValue(), short);
    const paragraph = 'We can pause, think, and keep speaking. The whole paragraph stays here. '.repeat(30);
    await recognition({ type:'partial', text:paragraph });
    await page.waitForFunction(text => document.querySelector('textarea').value === text, paragraph);
    await transcript.evaluate(e => { e.scrollTop = 0; e.dispatchEvent(new Event('scroll')); });
    await recognition({ type:'partial', text:paragraph + 'Another thought.' }); await page.waitForTimeout(180);
    assert.equal(await transcript.evaluate(e => e.scrollTop), 0);
    await page.getByRole('button', { name:'Follow latest words ↓' }).click(); assert(await transcript.evaluate(e => e.scrollTop > 0));
    await page.getByRole('button', { name:'Stop recording', exact:true }).click();
    await recognition({ type:'final', text:'Keep the thought moving.' }); await recognition({ type:'stopped' });
    // claude 2026-09-11: a finished session that could not be typed into now
    // reaches the clipboard on its own, so there is no Copy click to make here.
    await page.getByRole('button', { name:'Copied', exact:true }).waitFor();
    assert.equal(state.copied, 'Keep the thought moving.');
    assert.deepEqual(await transcript.boundingBox(), box);
    await transcript.fill('Edited final text.'); await page.getByRole('button', { name:'Copied', exact:true }).waitFor({ state:'hidden' });
    await page.getByRole('button', { name:'Copy text' }).click(); assert.equal(state.copied, 'Edited final text.');
    await page.getByRole('button', { name:'Start recording', exact:true }).click();
    await page.getByRole('button', { name:'Stop recording', exact:true }).waitFor();
    const canceledGeneration = state.generation;
    await recognition({ type:'partial', text:'Discard this pending stream of words.' });
    await page.getByRole('button', { name:'Cancel', exact:true }).click();
    await page.getByRole('button', { name:'Start recording', exact:true }).waitFor();
    await emit('recognition', { generation:canceledGeneration, event:{ type:'final', text:'Stale text must not return' } });
    await page.waitForTimeout(180); assert.equal(await transcript.inputValue(), '');
    await page.emulateMedia({ reducedMotion:'reduce' }); await overlay.emulateMedia({ reducedMotion:'reduce' });
    await page.getByRole('button', { name:'Start recording', exact:true }).click();
    await page.getByRole('button', { name:'Stop recording', exact:true }).waitFor();
    await page.evaluate(() => { window.renderedValues = []; });
    await recognition({ type:'partial', text:'Reduced motion shows received text immediately.' });
    await page.waitForFunction(() => document.querySelector('textarea').value === 'Reduced motion shows received text immediately.');
    assert.deepEqual(await page.evaluate(() => window.renderedValues.filter(Boolean)), ['Reduced motion shows received text immediately.']);
    await page.getByRole('button', { name:'Cancel', exact:true }).click();
    await page.setViewportSize({ width:600, height:600 });
    await page.getByRole('button', { name:'General', exact:true }).click();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    await page.goto(`${url}?fail-listener`);
    await page.getByText('Could not connect the shortcut. Quit and reopen Logia.').waitFor();
    assert.equal(await page.getByRole('button', { name:'Retry shortcut' }).count(), 0);
    assert.deepEqual(errors, []);
    console.log('Passed: settings/preferences, shortcut failure persistence, model state, deliberate test, progressive text/scroll, copy/edit, stale Cancel, reduced motion, narrow layout, listener failure.');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
