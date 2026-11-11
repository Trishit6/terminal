/**
 * Desktop account bridge.
 *
 * The Tauri Rust runtime performs authentication against the KERNEXT server and
 * keeps the session cookie in the operating system's credential store. The
 * webview only ever receives a public profile — never a session or CSRF token.
 */

import { invokeCommand } from './tauri';
import type {
  AuthSessionSummary,
  AuthStatus,
  AuthUser,
  Preferences,
  RegisterResult,
} from './auth';

export interface DesktopAuthState extends AuthStatus {
  signed_in: boolean;
  server_reachable: boolean;
  user?: AuthUser;
  preferences?: Preferences;
}

export interface DesktopRegisterResult extends RegisterResult {
  message: string;
}

export function desktopAuthState(): Promise<DesktopAuthState> {
  return invokeCommand<DesktopAuthState>('desktop_auth_state');
}

export function desktopAuthSignIn(
  email: string,
  password: string,
  remember: boolean,
): Promise<AuthUser> {
  return invokeCommand<AuthUser>('desktop_auth_sign_in', {
    email,
    password,
    remember,
  });
}

export function desktopAuthRegister(input: {
  fullName: string;
  email: string;
  password: string;
  acceptTerms: boolean;
}): Promise<DesktopRegisterResult> {
  return invokeCommand<DesktopRegisterResult>('desktop_auth_register', {
    fullName: input.fullName,
    email: input.email,
    password: input.password,
    acceptTerms: input.acceptTerms,
  });
}

export function desktopAuthSignOut(): Promise<void> {
  return invokeCommand<void>('desktop_auth_sign_out');
}

export function desktopAuthForgotPassword(
  email: string,
): Promise<{ message: string; email_delivery: string }> {
  return invokeCommand('desktop_auth_forgot_password', { email });
}

export function desktopAuthResetPassword(
  token: string,
  password: string,
): Promise<{ message: string }> {
  return invokeCommand('desktop_auth_reset_password', { token, password });
}

export function desktopAuthVerifyEmail(
  token: string,
): Promise<{ message: string }> {
  return invokeCommand('desktop_auth_verify_email', { token });
}

export function desktopAuthChangePassword(
  currentPassword: string,
  newPassword: string,
): Promise<{ message: string }> {
  return invokeCommand('desktop_auth_change_password', {
    currentPassword,
    newPassword,
  });
}

export function desktopAuthUpdateProfile(
  fullName: string,
  email: string,
): Promise<AuthUser> {
  return invokeCommand<AuthUser>('desktop_auth_update_profile', {
    fullName,
    email,
  });
}

export function desktopAuthSessions(): Promise<AuthSessionSummary[]> {
  return invokeCommand<AuthSessionSummary[]>('desktop_auth_sessions');
}

export function desktopAuthRevokeSession(id: string): Promise<void> {
  return invokeCommand<void>('desktop_auth_revoke_session', { id });
}

export function desktopAuthResendVerification(): Promise<{
  email_delivery: string;
}> {
  return invokeCommand('desktop_auth_resend_verification');
}

export function desktopAuthUpdatePreferences(
  theme: string,
): Promise<Preferences> {
  return invokeCommand<Preferences>('desktop_auth_update_preferences', {
    theme,
  });
}
