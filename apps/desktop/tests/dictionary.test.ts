import { test } from 'node:test';
import assert from 'node:assert/strict';
import { applyDictionary, ruleIssue, validatedRules, type DictionaryRule } from '../src/dictionary.ts';

const rule = (spoken: string, replacement: string, enabled = true): DictionaryRule => ({ spoken, replacement, enabled });

test('replaces the phrases the recognizer gets wrong, verbatim', () => {
  const rules = [rule('lobia', 'Logia'), rule('use state', 'useState'), rule('hard key', 'hotkey')];
  assert.equal(
    applyDictionary('Lobia keeps the use state hook behind a hard key.', rules),
    'Logia keeps the useState hook behind a hotkey.');
});

test('matches whole phrases only, never inside a longer word', () => {
  const rules = [rule('cat', 'dog')];
  assert.equal(applyDictionary('The category of cat is catalogued.', rules), 'The category of dog is catalogued.');
});

test('tolerates however the words were spaced', () => {
  assert.equal(applyDictionary('use   state and use\nstate', [rule('use state', 'useState')]), 'useState and useState');
});

test('phrases that begin or end with punctuation still match', () => {
  assert.equal(applyDictionary('Save it as login dot tsx now', [rule('dot tsx', '.tsx')]), 'Save it as login .tsx now');
});

test('disabled rules do nothing', () => {
  assert.equal(applyDictionary('Lobia', [rule('lobia', 'Logia', false)]), 'Lobia');
});

test('a longer phrase wins over a shorter overlapping one', () => {
  const rules = [rule('state', 'STATE'), rule('use state', 'useState')];
  assert.equal(applyDictionary('use state here', rules), 'useState here');
});

test('replacements are never re-matched by another rule', () => {
  // `a` -> `b` must not then be rewritten by `b` -> `c`.
  const rules = [rule('alpha', 'beta'), rule('beta', 'gamma')];
  assert.equal(applyDictionary('alpha', rules), 'beta');
});

test('nothing is invented: no rule means no change', () => {
  const text = 'uh can you create a react component for login page';
  assert.equal(applyDictionary(text, []), text);
  assert.equal(applyDictionary(text, [rule('nothing here', 'x')]), text);
});

test('matching ignores case but the replacement is inserted exactly', () => {
  assert.equal(applyDictionary('LOBIA and lobia', [rule('Lobia', 'Logia')]), 'Logia and Logia');
});

test('non-Latin phrases match on word boundaries too', () => {
  assert.equal(applyDictionary('मैं हिंदी बोलता हूँ', [rule('हिंदी', 'Hindi')]), 'मैं Hindi बोलता हूँ');
});

test('rejects blank, unchanged and duplicate rules', () => {
  const existing = [rule('lobia', 'Logia')];
  assert.equal(ruleIssue(rule('', 'x'), existing), 'blank-spoken');
  assert.equal(ruleIssue(rule('x', ''), existing), 'blank-replacement');
  assert.equal(ruleIssue(rule('same', 'same'), existing), 'unchanged');
  assert.equal(ruleIssue(rule('LOBIA', 'Logia'), existing), 'duplicate');
  assert.equal(ruleIssue(rule('hard key', 'hotkey'), existing), null);
});

test('a case-only change is a legitimate rule', () => {
  assert.equal(ruleIssue(rule('react', 'React'), []), null);
  assert.equal(applyDictionary('a react component', [rule('react', 'React')]), 'a React component');
});

test('malformed stored rules are discarded individually', () => {
  const stored = [
    { spoken: 'lobia', replacement: 'Logia', enabled: true },
    { spoken: '  ', replacement: 'x' },
    { spoken: 'y', replacement: '' },
    'not an object',
    { spoken: 'LOBIA', replacement: 'duplicate ignored' },
    { spoken: 'hard key', replacement: 'hotkey' }
  ];
  assert.deepEqual(validatedRules(stored), [
    { spoken: 'lobia', replacement: 'Logia', enabled: true },
    { spoken: 'hard key', replacement: 'hotkey', enabled: true }
  ]);
  assert.deepEqual(validatedRules(null), []);
  assert.deepEqual(validatedRules({ spoken: 'x' }), []);
});
