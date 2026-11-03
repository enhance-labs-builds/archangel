export function PriceTag({ cents }: { cents: number }) {
  return <span>{(cents / 100).toFixed(2)}</span>;
}
