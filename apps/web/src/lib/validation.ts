/** Shared client-side password rules; the server enforces the same rules. */
export const MIN_PASSWORD_LENGTH = 10;
export const MAX_PASSWORD_LENGTH = 128;

export function passwordProblem(password: string): string | null {
  if (password.length < MIN_PASSWORD_LENGTH) {
    return `Use at least ${MIN_PASSWORD_LENGTH} characters.`;
  }

  if (password.length > MAX_PASSWORD_LENGTH) {
    return `Use at most ${MAX_PASSWORD_LENGTH} characters.`;
  }

  if (!/[a-zA-Z]/.test(password)) {
    return 'Include at least one letter.';
  }

  if (!/[0-9]/.test(password)) {
    return 'Include at least one number.';
  }

  return null;
}

export function isValidEmail(email: string): boolean {
  const value = email.trim();
  return (
    value.length <= 320 &&
    /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)
  );
}
