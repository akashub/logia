import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readPreferences, savePreferences } from '../src/preferences.ts';

const defaults = { position: 'bottom', showIdle: false, theme: 'light', shortcut: 'Control+Alt+Space' };
function storage(value: string | null = null) {
  return { getItem: (_key: string) => value, setItem: (_key: string, next: string) => { value = next; } };
}

test('missing and damaged preferences recover without blocking startup', () => {
  for (const value of [null, '{broken', 'null', '[]', '42']) {
    assert.deepEqual(readPreferences(storage(value)), defaults);
  }
  assert.deepEqual(readPreferences({ getItem() { throw new Error('blocked'); }, setItem() {} }), defaults);
});

test('valid custom preferences survive saving and reopening', () => {
  const disk = storage();
  const selected = { position: 'top' as const, showIdle: false, theme: 'dark' as const, shortcut: 'Alt+Shift+Space' };
  savePreferences(selected, disk);
  assert.deepEqual(readPreferences(disk), selected);
});

test('invalid fields are discarded independently while valid choices survive', () => {
  assert.deepEqual(readPreferences(storage(JSON.stringify({
    position: 'left', showIdle: 'false', theme: 'dark', shortcut: 'Space', transcript: 'private words',
  }))), { ...defaults, theme: 'dark' });
  assert.deepEqual(readPreferences(storage(JSON.stringify({ position: 'top', shortcut: 'Control++Space' }))), {
    ...defaults, position: 'top',
  });
});

test('saved shortcuts must match a native-supported preset before startup registration', () => {
  for (const shortcut of ['Control+A', 'CommandOrControl+Shift+Space', 'Control+Control+Space']) {
    assert.deepEqual(readPreferences(storage(JSON.stringify({ theme: 'dark', shortcut }))), {
      ...defaults, theme: 'dark',
    });
  }
  for (const shortcut of ['Control+Alt+Space', 'Control+Shift+Space', 'Alt+Shift+Space']) {
    assert.equal(readPreferences(storage(JSON.stringify({ shortcut }))).shortcut, shortcut);
  }
});

test('saving excludes unrelated content and tolerates unavailable storage', () => {
  const disk = storage();
  savePreferences({ ...defaults, position: 'bottom', theme: 'light', transcript: 'private words' } as any, disk);
  const saved = disk.getItem('');
  assert.equal(typeof saved, 'string');
  assert.equal(saved!.includes('private words'), false);
  assert.doesNotThrow(() => savePreferences({ ...defaults, position: 'bottom', theme: 'light' }, {
    getItem: () => null, setItem() { throw new Error('quota'); },
  }));
});
