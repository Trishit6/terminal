import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import {
  AuthShell,
  FormError,
  FormField,
  FormNotice,
  SubmitButton,
} from '@/components/auth/auth-shell';
import { describeAuthError, useAuth } from '@/components/auth/auth-provider';
import { isValidEmail } from '@/lib/validation';

export function ForgotPasswordPage() {
  const { forgotPassword } = useAuth();
  const [email, setEmail] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldError, setFieldError] = useState<string | null>(null);
  const [result, setResult] = useState<{
    message: string;
    emailDelivery: string;
    devToken?: string;
  } | null>(null);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    if (!isValidEmail(email)) {
      setFieldError('Enter the email address for your account.');
      return;
    }

    setFieldError(null);
    setPending(true);
    try {
      const response = await forgotPassword(email.trim());
      setResult({
        message: response.message,
        emailDelivery: response.email_delivery,
        devToken: response.dev_reset_token,
      });
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setPending(false);
    }
  }

  return (
    <AuthShell
      title="Reset your password."
      description="Enter your account email and we'll issue a single-use reset link."
      footer={
        <>
          Remembered it?{' '}
          <Link to="/sign-in" className="text-primary hover:underline">
            Back to sign in
          </Link>
        </>
      }
    >
      {result ? (
        <div className="space-y-4">
          <FormNotice
            tone={result.emailDelivery === 'sent' ? 'success' : 'warning'}
          >
            {result.message}
          </FormNotice>

          {result.devToken ? (
            <div className="space-y-2">
              <p className="text-[11px] text-muted-foreground">
                Development mode: email delivery is disabled, so the reset link
                is shown here instead. It expires in one hour and works once.
              </p>
              <Link
                to="/reset-password"
                search={{ token: result.devToken }}
                className="block break-all rounded-xl border border-border bg-background/60 p-3 font-mono text-[11px] text-primary hover:border-primary/40"
              >
                Open password reset →
              </Link>
            </div>
          ) : null}

          {result.emailDelivery === 'not_configured' ? (
            <p className="text-[11px] leading-relaxed text-muted-foreground">
              This server has no SMTP configuration, so no message was sent. An
              administrator must configure SMTP_HOST, SMTP_PORT, SMTP_USERNAME,
              SMTP_PASSWORD and SMTP_FROM before password recovery can email
              links.
            </p>
          ) : null}
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="space-y-4" noValidate>
          <FormError>{error}</FormError>

          <FormField
            label="Email address"
            name="email"
            type="email"
            value={email}
            onChange={setEmail}
            autoComplete="email"
            placeholder="you@example.com"
            error={fieldError}
            disabled={pending}
          />

          <SubmitButton pending={pending}>Send reset link</SubmitButton>

          <p className="text-[11px] leading-relaxed text-muted-foreground">
            For privacy, the response is the same whether or not an account
            exists for that address.
          </p>
        </form>
      )}
    </AuthShell>
  );
}
