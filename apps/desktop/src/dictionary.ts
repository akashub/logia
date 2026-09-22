// claude 2026-09-11: R07 vocabulary rules.
//
// The recogniser is good at speech and bad at proper nouns: "Logia" comes back
// as "Lobia", "hotkey" as "hard key". These rules fix exactly that, and nothing
// else. Deliberate constraints, from docs/product-scope.md R07:
//
//   * Whole-phrase matches only. A rule never fires inside a longer word, so
//     "cat" cannot corrupt "category".
//   * Nothing is invented. No grammar, no articles, no punctuation guessing,
//     no automatic capitalisation. A rule replaces exactly what the user typed.
//   * Applied to final text only, never to live captions. Rewriting a caption
//     while the model is still revising it would fight the recogniser and make
//     the churn worse.
//   * The raw transcript is preserved so a rule can never lose speech.
//
// Matching is Unicode-aware and case-insensitive on the spoken phrase, because
// the recogniser's capitalisation is not reliable. The replacement is inserted
// verbatim — that is the whole point of a rule like `use state` → `useState`.

export type DictionaryRule = {
  /** What the recogniser produced, as the user hears themselves say it. */
  spoken: string;
  /** Exactly what should appear instead. Inserted verbatim. */
  replacement: string;
  enabled: boolean;
};

export type DictionaryIssue = 'blank-spoken' | 'blank-replacement' | 'duplicate' | 'unchanged';

const MAX_RULES = 200;
const MAX_FIELD = 120;

/** Letters, digits and marks count as word characters for boundary purposes. */
const WORD = /[\p{L}\p{N}\p{M}]/u;

function escape(value: string) { return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

/** Collapse runs of whitespace so a rule matches however the words were spaced. */
function pattern(spoken: string) {
  const parts = spoken.trim().split(/\s+/u).map(escape);
  return new RegExp(parts.join('\\s+'), 'giu');
}

function boundedAt(text: string, start: number, end: number) {
  const before = start > 0 ? text[start - 1] : '';
  const after = end < text.length ? text[end] : '';
  // A phrase that starts or ends with a non-word character (".tsx", "C++")
  // does not need a word boundary on that side.
  const needsBefore = WORD.test(text[start] ?? '');
  const needsAfter = WORD.test(text[end - 1] ?? '');
  if (needsBefore && before && WORD.test(before)) return false;
  if (needsAfter && after && WORD.test(after)) return false;
  return true;
}

/**
 * Reports why a rule cannot be saved, or null when it is usable.
 * `existing` should exclude the rule being edited.
 */
export function ruleIssue(rule: DictionaryRule, existing: DictionaryRule[]): DictionaryIssue | null {
  const spoken = rule.spoken.trim(), replacement = rule.replacement.trim();
  if (!spoken) return 'blank-spoken';
  if (!replacement) return 'blank-replacement';
  if (spoken.toLowerCase() === replacement.toLowerCase() && spoken === replacement) return 'unchanged';
  if (existing.some(other => other.spoken.trim().toLowerCase() === spoken.toLowerCase())) return 'duplicate';
  return null;
}

export function issueMessage(issue: DictionaryIssue) {
  if (issue === 'blank-spoken') return 'Enter the words as Logia hears them.';
  if (issue === 'blank-replacement') return 'Enter what should appear instead.';
  if (issue === 'duplicate') return 'A rule for those spoken words already exists.';
  return 'The replacement matches the spoken words, so this rule would change nothing.';
}

/**
 * Applies enabled rules to finalised text. Longer phrases win, so a specific
 * rule is never pre-empted by a shorter one that overlaps it. Each position is
 * rewritten at most once, so rules cannot chain into each other's output.
 */
export function applyDictionary(text: string, rules: DictionaryRule[]): string {
  if (!text) return text;
  const active = rules
    .filter(rule => rule.enabled && rule.spoken.trim() && rule.replacement.trim())
    .sort((a, b) => b.spoken.trim().length - a.spoken.trim().length)
    .slice(0, MAX_RULES);
  if (!active.length) return text;

  // Collect non-overlapping matches first, then rewrite once, right to left.
  const claimed: { start: number; end: number; value: string }[] = [];
  for (const rule of active) {
    const expression = pattern(rule.spoken);
    for (const match of text.matchAll(expression)) {
      const start = match.index ?? 0, end = start + match[0].length;
      if (!boundedAt(text, start, end)) continue;
      if (claimed.some(taken => start < taken.end && taken.start < end)) continue;
      claimed.push({ start, end, value: rule.replacement });
    }
  }
  claimed.sort((a, b) => b.start - a.start);
  let result = text;
  for (const { start, end, value } of claimed) result = result.slice(0, start) + value + result.slice(end);
  return result;
}

/** Discards anything malformed rather than failing the whole list. */
export function validatedRules(value: unknown): DictionaryRule[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const rules: DictionaryRule[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue;
    const record = entry as Record<string, unknown>;
    const spoken = typeof record.spoken === 'string' ? record.spoken.trim().slice(0, MAX_FIELD) : '';
    const replacement = typeof record.replacement === 'string' ? record.replacement.trim().slice(0, MAX_FIELD) : '';
    if (!spoken || !replacement) continue;
    const key = spoken.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    rules.push({ spoken, replacement, enabled: record.enabled !== false });
    if (rules.length >= MAX_RULES) break;
  }
  return rules;
}

// Storage lives here rather than in preferences.ts so a damaged rule list can
// never cost the user their other settings, and so neither module imports the
// other.
type RuleStorage = Pick<Storage, 'getItem' | 'setItem'>;
const storageKey = 'logia.dictionary.v1';

export function readDictionary(storage?: RuleStorage): DictionaryRule[] {
  try { return validatedRules(JSON.parse((storage ?? globalThis.localStorage).getItem(storageKey) ?? 'null')); }
  catch { return []; }
}

export function saveDictionary(rules: DictionaryRule[], storage?: RuleStorage) {
  try { (storage ?? globalThis.localStorage).setItem(storageKey, JSON.stringify(validatedRules(rules))); }
  catch { /* storage unavailable; the session keeps working without persistence */ }
}
