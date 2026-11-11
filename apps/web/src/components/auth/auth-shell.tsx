import { motion, useReducedMotion } from 'motion/react';
import { Eye, EyeOff, Loader2 } from 'lucide-react';
import { useId, useState } from 'react';
import { LogoMark } from '@/components/site/logo';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

/**
 * Shared chrome for every account screen.
 *
 * Deliberately restrained: one surface, a thin border, a single restrained
 * accent line. No split-screen marketing panel and no animated background.
 */
export function AuthShell({
  title,
  description,
  children,
  footer,
  wide = false,
}: {
  title: string;
  description: React.ReactNode;
  children: React.ReactNode;
  footer?: React.ReactNode;
  wide?: boolean;
}) {
  const reduced = useReducedMotion();

  return (
    <div className="relative flex min-h-[calc(100dvh-4rem)] flex-col overflow-hidden">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_60%_45%_at_50%_-10%,color-mix(in_srgb,var(--primary)_10%,transparent),transparent)]"
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/40 to-transparent"
      />

      <div className="relative mx-auto flex w-full max-w-6xl flex-1 items-center justify-center px-4 py-12 sm:px-6">
        <motion.div
          initial={reduced ? false : { opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
          className={cn('w-full', wide ? 'max-w-3xl' : 'max-w-[26rem]')}
        >
          <div className="mb-7 flex flex-col items-center text-center">
            <span className="inline-flex size-11 items-center justify-center rounded-2xl border border-border bg-surface shadow-[0_18px_50px_-30px_rgba(34,211,238,0.55)]">
              <LogoMark className="size-7" />
            </span>
            <span className="mt-4 font-mono text-[10px] uppercase tracking-[0.28em] text-muted-foreground">
              KERNEXT Account
            </span>
            <h1 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-[1.7rem]">
              {title}
            </h1>
            <p className="mt-2 max-w-sm text-sm leading-relaxed text-muted-foreground">
              {description}
            </p>
          </div>

          <div className="rounded-2xl border border-border bg-surface/85 p-6 shadow-[0_30px_100px_-50px_rgba(2,8,23,0.95)] backdrop-blur-sm sm:p-7">
            {children}
          </div>

          {footer ? (
            <div className="mt-5 text-center text-xs leading-relaxed text-muted-foreground">
              {footer}
            </div>
          ) : null}
        </motion.div>
      </div>
    </div>
  );
}

export function FormField({
  label,
  name,
  type = 'text',
  value,
  onChange,
  autoComplete,
  placeholder,
  error,
  hint,
  required = true,
  disabled = false,
  trailing,
}: {
  label: string;
  name: string;
  type?: string;
  value: string;
  onChange: (value: string) => void;
  autoComplete?: string;
  placeholder?: string;
  error?: string | null;
  hint?: React.ReactNode;
  required?: boolean;
  disabled?: boolean;
  trailing?: React.ReactNode;
}) {
  const id = useId();
  const helpId = `${id}-help`;
  const errorId = `${id}-error`;

  return (
    <div className="space-y-1.5">
      <div className="flex items-baseline justify-between gap-3">
        <label
          htmlFor={id}
          className="text-xs font-medium text-foreground/90"
        >
          {label}
        </label>
        {hint ? (
          <span id={helpId} className="text-[11px] text-muted-foreground">
            {hint}
          </span>
        ) : null}
      </div>
      <div className="relative">
        <input
          id={id}
          name={name}
          type={type}
          value={value}
          required={required}
          disabled={disabled}
          autoComplete={autoComplete}
          placeholder={placeholder}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? errorId : hint ? helpId : undefined}
          onChange={(event) => onChange(event.target.value)}
          className={cn(
            'h-11 w-full rounded-xl border bg-background/60 px-3.5 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground/60',
            'focus:border-primary/60 focus:ring-2 focus:ring-primary/15',
            trailing ? 'pr-11' : '',
            error
              ? 'border-danger/60 focus:border-danger focus:ring-danger/15'
              : 'border-border hover:border-border-strong',
          )}
        />
        {trailing ? (
          <div className="absolute inset-y-0 right-1 flex items-center">
            {trailing}
          </div>
        ) : null}
      </div>
      {error ? (
        <p id={errorId} className="text-[11px] leading-relaxed text-danger">
          {error}
        </p>
      ) : null}
    </div>
  );
}

export function PasswordField({
  label = 'Password',
  name,
  value,
  onChange,
  autoComplete,
  error,
  hint,
  placeholder,
  disabled = false,
}: {
  label?: string;
  name: string;
  value: string;
  onChange: (value: string) => void;
  autoComplete?: string;
  error?: string | null;
  hint?: React.ReactNode;
  placeholder?: string;
  disabled?: boolean;
}) {
  const [visible, setVisible] = useState(false);

  return (
    <FormField
      label={label}
      name={name}
      type={visible ? 'text' : 'password'}
      value={value}
      onChange={onChange}
      autoComplete={autoComplete}
      error={error}
      hint={hint}
      placeholder={placeholder}
      disabled={disabled}
      trailing={
        <button
          type="button"
          onClick={() => setVisible((current) => !current)}
          className="grid size-9 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-surface-soft hover:text-foreground"
          aria-label={visible ? 'Hide password' : 'Show password'}
          aria-pressed={visible}
          tabIndex={-1}
        >
          {visible ? (
            <EyeOff className="size-4" aria-hidden="true" />
          ) : (
            <Eye className="size-4" aria-hidden="true" />
          )}
        </button>
      }
    />
  );
}

export function FormError({ children }: { children?: React.ReactNode }) {
  if (!children) {
    return null;
  }

  return (
    <div
      role="alert"
      className="rounded-xl border border-danger/30 bg-danger/10 px-3.5 py-2.5 text-xs leading-relaxed text-danger"
    >
      {children}
    </div>
  );
}

export function FormNotice({
  children,
  tone = 'info',
}: {
  children: React.ReactNode;
  tone?: 'info' | 'success' | 'warning';
}) {
  const tones = {
    info: 'border-primary/25 bg-primary/8 text-foreground',
    success: 'border-success/30 bg-success/10 text-success',
    warning: 'border-warning/30 bg-warning/10 text-warning',
  } as const;

  return (
    <div
      role="status"
      className={cn(
        'rounded-xl border px-3.5 py-2.5 text-xs leading-relaxed',
        tones[tone],
      )}
    >
      {children}
    </div>
  );
}

export function SubmitButton({
  pending,
  children,
  className,
}: {
  pending: boolean;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Button
      type="submit"
      size="lg"
      disabled={pending}
      className={cn('w-full', className)}
    >
      {pending ? (
        <>
          <Loader2 className="size-4 animate-spin" aria-hidden="true" />
          Working…
        </>
      ) : (
        children
      )}
    </Button>
  );
}

export function AuthDivider({ children }: { children: React.ReactNode }) {
  return (
    <div className="relative my-5">
      <div className="absolute inset-0 flex items-center" aria-hidden="true">
        <div className="w-full border-t border-border" />
      </div>
      <div className="relative flex justify-center">
        <span className="bg-surface px-3 text-[11px] text-muted-foreground">
          {children}
        </span>
      </div>
    </div>
  );
}
