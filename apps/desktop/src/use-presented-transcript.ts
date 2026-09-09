import { useLayoutEffect, useRef, useState } from 'react';
import { ProgressiveTranscript } from './transcript-presentation';

export function usePresentedTranscript(text: string, streaming: boolean) {
  const presenter = useRef(new ProgressiveTranscript());
  const [shown, setShown] = useState(text);
  const [reducedMotion, setReducedMotion] = useState(() => matchMedia('(prefers-reduced-motion: reduce)').matches);
  useLayoutEffect(() => {
    const preference = matchMedia('(prefers-reduced-motion: reduce)');
    const changed = () => setReducedMotion(preference.matches);
    preference.addEventListener('change', changed);
    return () => preference.removeEventListener('change', changed);
  }, []);

  useLayoutEffect(() => {
    if (!streaming || reducedMotion || !text) {
      setShown(presenter.current.flush(text));
      return;
    }
    setShown(presenter.current.update(text, performance.now()));
    let frame = 0;
    const render = (now: number) => {
      const next = presenter.current.advance(now);
      setShown(next);
      if (next !== text) frame = requestAnimationFrame(render);
    };
    frame = requestAnimationFrame(render);
    return () => cancelAnimationFrame(frame);
  }, [text, streaming, reducedMotion]);

  // Final text, edits and Cancel take effect in the same render.
  return streaming && !reducedMotion ? shown : text;
}
