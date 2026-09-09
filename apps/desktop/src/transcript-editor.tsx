import { useLayoutEffect, useRef, useState } from 'react';
import { usePresentedTranscript } from './use-presented-transcript';

type Props = { text: string; active: boolean; streaming: boolean; onChange: (text: string) => void };

export function TranscriptEditor({ text, active, streaming, onChange }: Props) {
  const shown = usePresentedTranscript(text, streaming);
  const editor = useRef<HTMLTextAreaElement>(null);
  const following = useRef(true);
  const [showFollow, setShowFollow] = useState(false);
  const follow = () => {
    following.current = true;
    setShowFollow(false);
    if (editor.current) editor.current.scrollTop = editor.current.scrollHeight;
  };
  useLayoutEffect(() => {
    if (!text) follow();
    if (active && following.current && editor.current) editor.current.scrollTop = editor.current.scrollHeight;
  }, [shown, active, text]);

  return <div className="writing-surface">
    <textarea ref={editor} aria-label="Transcript" spellCheck={!active}
      value={shown} readOnly={active}
      onChange={event => onChange(event.target.value)}
      onScroll={event => {
        const element = event.currentTarget;
        following.current = element.scrollHeight - element.scrollTop - element.clientHeight < 48;
        setShowFollow(!following.current);
      }}
      placeholder="Start with a thought. Your words will appear here as you speak." />
    {active && showFollow && <button className="follow-latest" onClick={follow}>Follow latest words ↓</button>}
  </div>;
}
