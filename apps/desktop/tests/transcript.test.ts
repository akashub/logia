import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ProgressiveTranscript } from '../src/transcript-presentation.ts';

test('reveals only received words and catches up within 140 ms', () => {
  const view = new ProgressiveTranscript();
  const target = 'A whole paragraph arriving together.';
  view.update(target, 0);
  const middle = view.advance(70);
  assert(middle.length > 0 && middle.length < target.length);
  assert(target.startsWith(middle));
  assert.equal(view.advance(140), target);
});

test('revisions replace visible words without erasing and retyping the prefix', () => {
  const view = new ProgressiveTranscript();
  view.flush('Meet me on Tuesday');
  view.update('Meet me this Friday morning.', 100);
  assert.equal(view.advance(100), 'Meet me this Friday');
  assert.equal(view.advance(240), 'Meet me this Friday morning.');
});

test('a newer hypothesis supersedes pending words', () => {
  const view = new ProgressiveTranscript();
  view.update('One wrong sentence with a long tail.', 0);
  view.advance(35);
  view.update('One correct sentence.', 40);
  assert.equal(view.advance(180), 'One correct sentence.');
  assert.equal(view.advance(1000), 'One correct sentence.');
});

test('finalization and cancel flush pending output immediately', () => {
  const view = new ProgressiveTranscript();
  view.update('This should never reappear after cancel.', 0);
  assert.equal(view.flush(''), '');
  assert.equal(view.advance(1000), '');
  view.update('A provisional guess', 1100);
  assert.equal(view.flush('The authoritative final.'), 'The authoritative final.');
  assert.equal(view.advance(2000), 'The authoritative final.');
});

test('preserves unicode, punctuation and paragraph breaks exactly', () => {
  const view = new ProgressiveTranscript();
  const target = 'Café 🙂\n\nA second paragraph.\n';
  view.update(target, 0);
  assert.equal(view.advance(140), target);
});
