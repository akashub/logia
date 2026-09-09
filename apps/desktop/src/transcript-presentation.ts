// Presentation only: never predict words or change the authoritative transcript.
// Each received update is fully visible within 140 ms; no unbounded typing queue.
export class ProgressiveTranscript {
  private words: string[] = [];
  private shown = '';
  private from = 0;
  private started = 0;

  update(text: string, now: number) {
    const visible = this.shown.match(/\s*\S+|\s+$/gu)?.length ?? 0;
    this.words = text.match(/\s*\S+|\s+$/gu) ?? [];
    // Correct already-visible words in place. Only new words are paced out.
    this.from = Math.min(Math.max(visible, 1), this.words.length);
    this.started = now;
    this.shown = this.words.slice(0, this.from).join('');
    return this.shown;
  }

  advance(now: number) {
    const fraction = Math.min(1, Math.max(0, (now - this.started) / 140));
    const count = this.from + Math.floor((this.words.length - this.from) * fraction);
    this.shown = this.words.slice(0, count).join('');
    return this.shown;
  }

  flush(text: string) {
    this.update(text, 0);
    this.from = this.words.length;
    return this.shown = text;
  }
}
