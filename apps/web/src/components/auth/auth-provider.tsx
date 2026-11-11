import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import {
  AuthError,
  changePasswordRequest,
  describeAuthError,
  fetchAuthStatus,
  fetchMe,
  forgotPasswordRequest,
  hasSessionCookie,
  listSessionsRequest,
  registerRequest,
  resendVerificationRequest,
  resetPasswordRequest,
  revokeSessionRequest,
  signInRequest,
  signOutRequest,
  updatePreferencesRequest,
  updateProfileRequest,
  verifyEmailRequest,
  type AuthSessionSummary,
  type AuthStatus,
  type AuthUser,
  type Preferences,
  type RegisterResult,
} from '@/lib/auth';
import {
  desktopAuthForgotPassword,
  desktopAuthRegister,
  desktopAuthResendVerification,
  desktopAuthResetPassword,
  desktopAuthRevokeSession,
  desktopAuthSessions,
  desktopAuthSignIn,
  desktopAuthSignOut,
  desktopAuthState,
  desktopAuthUpdatePreferences,
  desktopAuthUpdateProfile,
  desktopAuthVerifyEmail,
  desktopAuthChangePassword,
} from '@/lib/desktop-auth';
import { isTauri } from '@/lib/tauri';

interface ProfileResult {
  user: AuthUser;
  email_verification_required: boolean;
  email_delivery: string;
  dev_verification_token?: string;
}

interface AuthContextValue {
  ready: boolean;
  signedIn: boolean;
  user: AuthUser | null;
  preferences: Preferences | null;
  accountsEnabled: boolean;
  emailConfigured: boolean;
  devMail: boolean;
  serverReachable: boolean;
  refresh: () => Promise<void>;
  signIn: (email: string, password: string, remember: boolean) => Promise<void>;
  register: (input: {
    fullName: string;
    email: string;
    password: string;
    acceptTerms: boolean;
  }) => Promise<RegisterResult>;
  signOut: () => Promise<void>;
  updateProfile: (fullName: string, email: string) => Promise<ProfileResult>;
  changePassword: (
    currentPassword: string,
    newPassword: string,
  ) => Promise<string>;
  listSessions: () => Promise<AuthSessionSummary[]>;
  revokeSession: (id: string) => Promise<void>;
  resendVerification: () => Promise<{ email_delivery: string }>;
  updatePreferences: (
    theme: string,
    terminalPreferences?: Record<string, unknown> | null,
  ) => Promise<Preferences>;
  forgotPassword: (email: string) => Promise<{
    message: string;
    email_delivery: string;
    dev_reset_token?: string;
  }>;
  resetPassword: (token: string, password: string) => Promise<string>;
  verifyEmail: (token: string) => Promise<string>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

const DISABLED: AuthStatus = {
  accounts_enabled: false,
  email_configured: false,
  dev_mail: false,
};

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [ready, setReady] = useState(false);
  const [user, setUser] = useState<AuthUser | null>(null);
  const [preferences, setPreferences] = useState<Preferences | null>(null);
  const [status, setStatus] = useState<AuthStatus>(DISABLED);
  const [serverReachable, setServerReachable] = useState(true);
  const preferencesRef = useRef(preferences);
  preferencesRef.current = preferences;

  const refresh = useCallback(async () => {
    if (isTauri()) {
      try {
        const state = await desktopAuthState();
        setStatus({
          accounts_enabled: state.accounts_enabled,
          email_configured: state.email_configured,
          dev_mail: state.dev_mail,
        });
        setServerReachable(state.server_reachable);
        setUser(state.signed_in ? (state.user ?? null) : null);
        setPreferences(state.user ? (state.preferences ?? null) : null);
      } catch {
        setServerReachable(false);
        setUser(null);
        setPreferences(null);
      } finally {
        setReady(true);
      }
      return;
    }

    try {
      const next = await fetchAuthStatus();
      setStatus(next);
      setServerReachable(true);

      if (!next.accounts_enabled) {
        setUser(null);
        setPreferences(null);
        return;
      }

      // Skip the authenticated probe for anonymous visitors: the CSRF
      // cookie only exists while a session cookie does, so without it the
      // `/api/auth/me` request can only answer 401. An expired session still
      // surfaces its genuine 401 here.
      if (!hasSessionCookie()) {
        setUser(null);
        setPreferences(null);
        return;
      }

      try {
        const me = await fetchMe();
        setUser(me.user);
        setPreferences(me.preferences);
      } catch (error) {
        if (error instanceof AuthError && error.status === 401) {
          setUser(null);
          setPreferences(null);
          return;
        }
        throw error;
      }
    } catch {
      setServerReachable(false);
      setUser(null);
      setPreferences(null);
    } finally {
      setReady(true);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const signIn = useCallback(
    async (email: string, password: string, remember: boolean) => {
      if (isTauri()) {
        const next = await desktopAuthSignIn(email, password, remember);
        setUser(next);
        await refresh();
        return;
      }

      await signInRequest(email, password, remember);
      await refresh();
    },
    [refresh],
  );

  const register = useCallback(
    async (input: {
      fullName: string;
      email: string;
      password: string;
      acceptTerms: boolean;
    }) => {
      if (isTauri()) {
        return desktopAuthRegister(input);
      }

      const result = await registerRequest(input);
      await refresh();
      return result;
    },
    [refresh],
  );

  const signOut = useCallback(async () => {
    if (isTauri()) {
      await desktopAuthSignOut();
    } else {
      try {
        await signOutRequest();
      } finally {
        // Even if the request fails the UI must stop showing a signed-in
        // account; the server remains the source of truth on next refresh.
      }
    }

    setUser(null);
    setPreferences(null);
  }, []);

  const updateProfile = useCallback(
    async (fullName: string, email: string) => {
      if (isTauri()) {
        const next = await desktopAuthUpdateProfile(fullName, email);
        setUser(next);
        return {
          user: next,
          email_verification_required: next.email !== user?.email,
          email_delivery: 'unknown',
        };
      }

      const result = await updateProfileRequest(fullName, email);
      setUser(result.user);
      return result;
    },
    [user?.email],
  );

  const changePassword = useCallback(
    async (currentPassword: string, newPassword: string) => {
      if (isTauri()) {
        const result = await desktopAuthChangePassword(
          currentPassword,
          newPassword,
        );
        return result.message;
      }

      const result = await changePasswordRequest(currentPassword, newPassword);
      return result.message;
    },
    [],
  );

  const listSessions = useCallback(async () => {
    if (isTauri()) {
      return desktopAuthSessions();
    }
    return listSessionsRequest();
  }, []);

  const revokeSession = useCallback(async (id: string) => {
    if (isTauri()) {
      await desktopAuthRevokeSession(id);
      return;
    }
    await revokeSessionRequest(id);
  }, []);

  const resendVerification = useCallback(async () => {
    if (isTauri()) {
      return desktopAuthResendVerification();
    }
    return resendVerificationRequest();
  }, []);

  const updatePreferences = useCallback(
    async (
      theme: string,
      terminalPreferences =
        preferencesRef.current?.terminal_preferences ?? null,
    ) => {
      const next = isTauri()
        ? await desktopAuthUpdatePreferences(theme)
        : await updatePreferencesRequest(theme, terminalPreferences);
      setPreferences(next);
      return next;
    },
    [],
  );

  const forgotPassword = useCallback(async (email: string) => {
    if (isTauri()) {
      return desktopAuthForgotPassword(email);
    }
    return forgotPasswordRequest(email);
  }, []);

  const resetPassword = useCallback(async (token: string, password: string) => {
    if (isTauri()) {
      const result = await desktopAuthResetPassword(token, password);
      return result.message;
    }
    const result = await resetPasswordRequest(token, password);
    return result.message;
  }, []);

  const verifyEmail = useCallback(
    async (token: string) => {
      const result = isTauri()
        ? await desktopAuthVerifyEmail(token)
        : await verifyEmailRequest(token);
      if (user) {
        await refresh();
      }
      return result.message;
    },
    [refresh, user],
  );

  const value = useMemo<AuthContextValue>(
    () => ({
      ready,
      signedIn: user !== null,
      user,
      preferences,
      accountsEnabled: status.accounts_enabled,
      emailConfigured: status.email_configured,
      devMail: status.dev_mail,
      serverReachable,
      refresh,
      signIn,
      register,
      signOut,
      updateProfile,
      changePassword,
      listSessions,
      revokeSession,
      resendVerification,
      updatePreferences,
      forgotPassword,
      resetPassword,
      verifyEmail,
    }),
    [
      ready,
      user,
      preferences,
      status,
      serverReachable,
      refresh,
      signIn,
      register,
      signOut,
      updateProfile,
      changePassword,
      listSessions,
      revokeSession,
      resendVerification,
      updatePreferences,
      forgotPassword,
      resetPassword,
      verifyEmail,
    ],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);

  if (!context) {
    throw new Error('useAuth must be used inside an AuthProvider');
  }

  return context;
}

export { describeAuthError };
