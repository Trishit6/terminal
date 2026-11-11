import { Link, useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import {
  AuthShell,
  FormError,
  FormField,
  FormNotice,
  PasswordField,
  SubmitButton,
} from '@/components/auth/auth-shell';
import { Checkbox } from '@/components/auth/checkbox';
import { describeAuthError, useAuth } from '@/components/auth/auth-provider';
import { passwordProblem } from '@/lib/validation';

export function CreateAccountPage() {
  const navigate = useNavigate();
  const { register, accountsEnabled } = useAuth();

  const [fullName, setFullName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [acceptTerms, setAcceptTerms] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [created, setCreated] = useState<{
    emailDelivery: string;
    devToken?: string;
  } | null>(null);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    const next: Record<string, string> = {};
    if (fullName.trim().length < 2) {
      next.fullName = 'Enter your full name.';
    }
    if (!email.trim() || !email.includes('@') || !email.includes('.')) {
      next.email = 'Enter a valid email address.';
    }
    const problem = passwordProblem(password);
    if (problem) {
      next.password = problem;
    }
    if (confirm !== password) {
      next.confirm = 'The passwords do not match.';
    }
    if (!acceptTerms) {
      next.terms = 'Accept the terms to create an account.';
    }
    setFieldErrors(next);

    if (Object.keys(next).length > 0) {
      return;
    }

    setPending(true);
    try {
      const result = await register({
        fullName: fullName.trim(),
        email: email.trim(),
        password,
        acceptTerms,
      });
      setCreated({
        emailDelivery: result.email_delivery,
        devToken: result.dev_verification_token,
      });
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setPending(false);
    }
  }

  if (created) {
    return (
      <AuthShell
        title="Your account is ready."
        description={`Signed in as ${email.trim()}. ${
          created.emailDelivery === 'sent'
            ? 'Check your inbox for a verification link.'
            : 'Email verification is pending.'
        }`}
        footer={
          <Link to="/account" className="text-primary hover:underline">
            Continue to your account
          </Link>
        }
      >
        <div className="space-y-4">
          {created.emailDelivery === 'sent' ? (
            <FormNotice tone="success">
              A verification email is on its way. You can keep working while it
              arrives.
            </FormNotice>
          ) : created.emailDelivery === 'development' && created.devToken ? (
            <div className="space-y-2">
              <FormNotice tone="warning">
                Email delivery is not configured. This development server issued
                a verification token directly.
              </FormNotice>
              <Link
                to="/verify-email"
                search={{ token: created.devToken }}
                className="block break-all rounded-xl border border-border bg-background/60 p-3 font-mono text-[11px] text-primary hover:border-primary/40"
              >
                Verify this email address →
              </Link>
            </div>
          ) : (
            <FormNotice tone="warning">
              This server has no email delivery configured, so no verification
              message was sent. You can resend verification later from your
              account once SMTP is configured.
            </FormNotice>
          )}

          <button
            type="button"
            onClick={() => void navigate({ to: '/account' })}
            className="h-11 w-full rounded-xl bg-primary text-sm font-medium text-[#03131a] transition-colors hover:bg-primary-strong"
          >
            Continue to account
          </button>
        </div>
      </AuthShell>
    );
  }

  return (
    <AuthShell
      title="Create your account."
      description="One account for your KERNEXT profile and protected web terminal sessions."
      footer={
        <>
          Already have an account?{' '}
          <Link
            to="/sign-in"
            className="text-foreground underline-offset-4 hover:text-primary hover:underline"
          >
            Sign in
          </Link>
        </>
      }
    >
      <form onSubmit={handleSubmit} className="space-y-4" noValidate>
        {!accountsEnabled ? (
          <FormNotice tone="warning">
            Account creation is not configured on this server. Set DATABASE_URL
            to a MariaDB connection and restart the KERNEXT server.
          </FormNotice>
        ) : null}

        <FormError>{error}</FormError>

        <FormField
          label="Full name"
          name="full_name"
          value={fullName}
          onChange={setFullName}
          autoComplete="name"
          placeholder="Ada Lovelace"
          error={fieldErrors.fullName}
          disabled={!accountsEnabled || pending}
        />

        <FormField
          label="Email address"
          name="email"
          type="email"
          value={email}
          onChange={setEmail}
          autoComplete="email"
          placeholder="you@example.com"
          error={fieldErrors.email}
          disabled={!accountsEnabled || pending}
        />

        <PasswordField
          name="password"
          value={password}
          onChange={setPassword}
          autoComplete="new-password"
          placeholder="At least 10 characters"
          error={fieldErrors.password}
          hint="10+ characters, a letter and a number"
          disabled={!accountsEnabled || pending}
        />

        <PasswordField
          label="Confirm password"
          name="confirm_password"
          value={confirm}
          onChange={setConfirm}
          autoComplete="new-password"
          placeholder="Repeat your password"
          error={fieldErrors.confirm}
          disabled={!accountsEnabled || pending}
        />

        <div className="space-y-1.5">
          <Checkbox
            checked={acceptTerms}
            onChange={setAcceptTerms}
            disabled={!accountsEnabled || pending}
            label={
              <>
                I agree to the KERNEXT Terms of Service and Privacy Policy, and
                to keeping my account credentials private.
              </>
            }
          />
          {fieldErrors.terms ? (
            <p className="text-[11px] text-danger">{fieldErrors.terms}</p>
          ) : null}
        </div>

        <SubmitButton pending={pending}>Create account</SubmitButton>
      </form>
    </AuthShell>
  );
}
