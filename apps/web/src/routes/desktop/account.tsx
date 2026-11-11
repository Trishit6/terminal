import { AccountPage } from '@/routes/auth/account';

/**
 * Account screen inside the desktop shell.
 *
 * Account data is optional: the local terminal, files, and settings remain
 * usable while signed out or offline.
 */
export function DesktopAccountPage() {
  return <AccountPage />;
}
