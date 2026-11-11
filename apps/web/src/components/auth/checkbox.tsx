import { Check } from 'lucide-react';

/** Minimal accessible checkbox used by the account forms. */
export function Checkbox({
  checked,
  onChange,
  label,
  disabled = false,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: React.ReactNode;
  disabled?: boolean;
}) {
  return (
    <label className="inline-flex cursor-pointer items-start gap-2.5 text-xs leading-relaxed text-muted-foreground">
      <span className="relative mt-0.5 flex size-4 shrink-0 items-center justify-center">
        <input
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={(event) => onChange(event.target.checked)}
          className="peer size-4 appearance-none rounded-[5px] border border-border-strong bg-background/60 transition-colors checked:border-primary checked:bg-primary disabled:opacity-50"
        />
        <Check
          aria-hidden="true"
          className="pointer-events-none absolute size-3 text-[#03131a] opacity-0 transition-opacity peer-checked:opacity-100"
        />
      </span>
      <span>{label}</span>
    </label>
  );
}
