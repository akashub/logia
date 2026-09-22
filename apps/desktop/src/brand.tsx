import './brand.css';

export function BrandMark({ className = '' }: { className?: string }) {
  return <span className={`logia-symbol ${className}`} aria-hidden="true" />;
}
export function Brand() {
  return <div className="wordmark"><BrandMark />Logia</div>;
}
