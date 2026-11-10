import { useCallback, useEffect, useState } from 'react';
import { Link } from '@tanstack/react-router';
import {
  Cpu,
  Monitor,
  RefreshCw,
  Trash2,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  desktopDeleteSession,
  desktopListSessions,
  desktopPtyClose,
  type DesktopSessionSummary,
} from '@/lib/desktop';

export function DesktopSessionsPage() {
  const [sessions, setSessions] = useState<DesktopSessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const next = await desktopListSessions();
      setSessions(next);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const closeSession = useCallback(
    async (session: DesktopSessionSummary) => {
      try {
        if (session.kind === 'engine') {
          await desktopDeleteSession(session.id);
        } else {
          await desktopPtyClose(session.id);
        }

        await load();
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
      }
    },
    [load],
  );

  return (
    <div className="mx-auto w-full max-w-[1400px] px-6 py-8 lg:px-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.22em] text-primary">
            Sessions
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Live sessions
          </h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            Every engine and PTY session owned by the desktop runtime.
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={() => void load()} className="gap-2">
          <RefreshCw className={loading ? 'size-3.5 animate-spin' : 'size-3.5'} />
          Refresh
        </Button>
      </div>

      {error ? (
        <div className="mt-6 rounded-lg border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger">
          {error}
        </div>
      ) : null}

      <div className="mt-8 space-y-4">
        {sessions.length === 0 ? (
          <Card>
            <CardContent className="py-10 text-center text-sm text-muted-foreground">
              No live sessions. Open the terminal to start one.
            </CardContent>
          </Card>
        ) : (
          sessions.map((session) => (
            <Card key={`${session.kind}-${session.id}`}>
              <CardHeader className="gap-2">
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2">
                    {session.kind === 'engine' ? (
                      <Cpu className="size-4 text-primary" />
                    ) : (
                      <Monitor className="size-4 text-primary" />
                    )}
                    <CardTitle className="text-base">
                      {session.kind === 'engine' ? 'Engine' : 'PTY'} session{' '}
                      {session.id}
                    </CardTitle>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant={session.state === 'running' ? 'success' : 'secondary'}>
                      {session.state}
                    </Badge>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => void closeSession(session)}
                      aria-label={`Close session ${session.id}`}
                      className="size-8 text-muted-foreground hover:text-danger"
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  </div>
                </div>
                <CardDescription className="font-mono text-xs">
                  {session.working_directory}
                </CardDescription>
              </CardHeader>
              <CardContent className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-4">
                <div>
                  <span className="font-medium text-foreground">Platform</span>
                  <div className="mt-0.5 font-mono">{session.platform}</div>
                </div>
                <div>
                  <span className="font-medium text-foreground">Version</span>
                  <div className="mt-0.5 font-mono">{session.version}</div>
                </div>
                <div>
                  <span className="font-medium text-foreground">Jobs</span>
                  <div className="mt-0.5 font-mono">{session.job_count}</div>
                </div>
                <div>
                  <span className="font-medium text-foreground">Prompt</span>
                  <div className="mt-0.5 font-mono">{session.prompt}</div>
                </div>
              </CardContent>
            </Card>
          ))
        )}
      </div>

      <div className="mt-8 flex justify-end">
        <Button asChild>
          <Link to="/desktop/terminal">Open a terminal session</Link>
        </Button>
      </div>
    </div>
  );
}
