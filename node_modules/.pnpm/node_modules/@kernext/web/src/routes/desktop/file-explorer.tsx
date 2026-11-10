import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ChevronRight,
  FolderOpen,
  Home,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  Trash2,
  ExternalLink,
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  desktopFsCreateDirectory,
  desktopFsDelete,
  desktopFsList,
  desktopFsOpen,
  desktopFsRename,
  desktopFsSessionDirectory,
  desktopListSessions,
  type DesktopSessionSummary,
  type DirectoryEntry,
  type DirectoryListing,
} from '@/lib/desktop';
import { cn } from '@/lib/utils';

export function DesktopFileExplorerPage() {
  const [listing, setListing] = useState<DirectoryListing | null>(null);
  const [path, setPath] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState<DirectoryEntry | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState('');
  const [renameTarget, setRenameTarget] = useState<DirectoryEntry | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<DirectoryEntry | null>(null);
  const [deleteRecursive, setDeleteRecursive] = useState(false);
  const [sessions, setSessions] = useState<DesktopSessionSummary[]>([]);
  const [sessionPickerOpen, setSessionPickerOpen] = useState(false);

  const load = useCallback(
    async (target?: string) => {
      setLoading(true);
      setError(null);

      try {
        const next = await desktopFsList(target);
        setListing(next);
        setPath(next.path);
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    void desktopListSessions()
      .then(setSessions)
      .catch(() => setSessions([]));
  }, []);

  const entries = useMemo(() => {
    const needle = query.trim().toLowerCase();

    if (!listing) {
      return [];
    }

    if (!needle) {
      return listing.entries;
    }

    return listing.entries.filter((entry) =>
      entry.name.toLowerCase().includes(needle),
    );
  }, [listing, query]);

  const breadcrumb = useMemo(() => {
    if (!path) {
      return [];
    }

    const parts = path.split(/[\\/]+/).filter(Boolean);
    return parts.map((part, index) => ({
      label: part,
      path: parts.slice(0, index + 1).join('\\'),
    }));
  }, [path]);

  const openDirectory = useCallback(
    async (entry: DirectoryEntry) => {
      if (!entry.is_dir) {
        return;
      }

      setSelected(null);
      await load(entry.path);
    },
    [load],
  );

  const openEntry = useCallback(
    async (entry: DirectoryEntry) => {
      try {
        if (entry.is_dir) {
          await openDirectory(entry);
        } else {
          await desktopFsOpen(entry.path);
        }
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
      }
    },
    [openDirectory],
  );

  const submitCreate = useCallback(async () => {
    const name = createName.trim();

    if (!name || !listing) {
      return;
    }

    const separator = path.includes('/') ? '/' : '\\';
    const target = `${listing.path}${separator}${name}`;

    try {
      await desktopFsCreateDirectory(target);
      setCreateOpen(false);
      setCreateName('');
      await load(listing.path);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, [createName, listing, load, path]);

  const submitRename = useCallback(async () => {
    if (!renameTarget || !listing) {
      return;
    }

    const name = renameValue.trim();

    if (!name) {
      return;
    }

    const separator = path.includes('/') ? '/' : '\\';
    const target = `${listing.path}${separator}${name}`;

    try {
      await desktopFsRename(renameTarget.path, target);
      setRenameTarget(null);
      setRenameValue('');
      await load(listing.path);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, [listing, load, path, renameTarget, renameValue]);

  const submitDelete = useCallback(async () => {
    if (!deleteTarget || !listing) {
      return;
    }

    try {
      await desktopFsDelete(deleteTarget.path, deleteRecursive);
      setDeleteTarget(null);
      setDeleteRecursive(false);
      await load(listing.path);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, [deleteRecursive, deleteTarget, listing, load]);

  const useSessionDirectory = useCallback(
    async (session: DesktopSessionSummary) => {
      try {
        const directory = await desktopFsSessionDirectory(session.id, session.kind);
        setSessionPickerOpen(false);
        await load(directory);
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
      }
    },
    [load],
  );

  return (
    <div className="mx-auto w-full max-w-site px-6 py-8 lg:px-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.22em] text-primary">
            File explorer
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Files
          </h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            Browse, inspect, and manage the filesystem with the permissions of
            the current user.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={() => void load(path)} className="gap-2">
            <RefreshCw className={loading ? 'size-3.5 animate-spin' : 'size-3.5'} />
            Refresh
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setSessionPickerOpen(true)}
            className="gap-2"
          >
            <FolderOpen className="size-3.5" />
            Session directory
          </Button>
          <Button size="sm" onClick={() => setCreateOpen(true)} className="gap-2">
            <Plus className="size-3.5" />
            New folder
          </Button>
        </div>
      </div>

      {error ? (
        <div className="mt-6 rounded-lg border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger">
          {error}
        </div>
      ) : null}

      <div className="mt-6 flex flex-col gap-3 lg:flex-row lg:items-center">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filter entries in this directory…"
            className="h-10 w-full rounded-lg border border-border bg-surface/70 pl-9 pr-3 text-sm outline-none transition-colors placeholder:text-muted-foreground focus:border-primary/50"
          />
        </div>
        <div className="flex items-center gap-2 rounded-lg border border-border bg-surface/70 px-3 py-2 text-xs">
          <Home className="size-3.5 text-primary" />
          <span className="font-mono text-muted-foreground">
            {listing ? `${listing.total} entries` : '—'}
          </span>
        </div>
      </div>

      <div className="mt-4 flex flex-wrap items-center gap-1 rounded-lg border border-border bg-surface/60 px-3 py-2 text-xs">
        <button
          type="button"
          onClick={() => void load()}
          className="rounded px-1.5 py-0.5 font-mono text-muted-foreground transition-colors hover:text-foreground"
        >
          root
        </button>
        {breadcrumb.map((part, index) => (
          <span key={part.path} className="flex items-center gap-1">
            <ChevronRight className="size-3 text-muted-foreground" />
            <button
              type="button"
              onClick={() => void load(part.path)}
              className={cn(
                'rounded px-1.5 py-0.5 font-mono transition-colors',
                index === breadcrumb.length - 1
                  ? 'text-foreground'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {part.label}
            </button>
          </span>
        ))}
      </div>

      <Card className="mt-4 overflow-hidden">
        <CardHeader className="gap-2">
          <CardTitle className="text-base">Directory contents</CardTitle>
          <CardDescription className="font-mono text-xs">
            {listing?.path ?? '—'}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <div className="overflow-x-auto">
            <table className="w-full min-w-[720px] text-left text-sm">
              <thead className="bg-surface-soft text-xs uppercase tracking-[0.14em] text-muted-foreground">
                <tr>
                  <th className="px-4 py-3 font-medium">Name</th>
                  <th className="px-4 py-3 font-medium">Type</th>
                  <th className="px-4 py-3 font-medium">Size</th>
                  <th className="px-4 py-3 font-medium">Modified</th>
                  <th className="px-4 py-3 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {entries.map((entry) => (
                  <tr
                    key={entry.path}
                    onClick={() => setSelected(entry)}
                    onDoubleClick={() => void openEntry(entry)}
                    className={cn(
                      'cursor-pointer transition-colors',
                      selected?.path === entry.path
                        ? 'bg-primary/10'
                        : 'bg-surface/40 hover:bg-surface/70',
                    )}
                  >
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-2">
                        {entry.is_dir ? (
                          <FolderOpen className="size-4 text-primary" />
                        ) : (
                          <ExternalLink className="size-4 text-muted-foreground" />
                        )}
                        <span className="font-mono text-xs">{entry.name}</span>
                      </div>
                    </td>
                    <td className="px-4 py-3">
                      <Badge variant={entry.is_dir ? 'default' : 'secondary'}>
                        {entry.is_dir ? 'folder' : 'file'}
                      </Badge>
                    </td>
                    <td className="px-4 py-3 font-mono text-xs text-muted-foreground">
                      {entry.is_dir ? '—' : formatSize(entry.size ?? 0)}
                    </td>
                    <td className="px-4 py-3 font-mono text-xs text-muted-foreground">
                      {entry.modified ?? '—'}
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex items-center justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={(event) => {
                            event.stopPropagation();
                            void openEntry(entry);
                          }}
                          aria-label={`Open ${entry.name}`}
                          className="size-8"
                        >
                          <ExternalLink className="size-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={(event) => {
                            event.stopPropagation();
                            setRenameTarget(entry);
                            setRenameValue(entry.name);
                          }}
                          aria-label={`Rename ${entry.name}`}
                          className="size-8"
                        >
                          <Pencil className="size-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={(event) => {
                            event.stopPropagation();
                            setDeleteTarget(entry);
                            setDeleteRecursive(false);
                          }}
                          aria-label={`Delete ${entry.name}`}
                          className="size-8 text-muted-foreground hover:text-danger"
                        >
                          <Trash2 className="size-4" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {!loading && entries.length === 0 ? (
            <div className="py-12 text-center text-sm text-muted-foreground">
              This directory is empty or no entry matches the filter.
            </div>
          ) : null}
        </CardContent>
      </Card>

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create a new folder</DialogTitle>
            <DialogDescription>
              The folder is created inside {listing?.path}.
            </DialogDescription>
          </DialogHeader>
          <input
            type="text"
            value={createName}
            onChange={(event) => setCreateName(event.target.value)}
            placeholder="folder name"
            className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 text-sm outline-none focus:border-primary/50"
          />
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateOpen(false)}>
              Cancel
            </Button>
            <Button onClick={() => void submitCreate()} disabled={!createName.trim()}>
              Create
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={renameTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            setRenameTarget(null);
            setRenameValue('');
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Rename {renameTarget?.name}</DialogTitle>
            <DialogDescription>
              The entry stays in the same directory.
            </DialogDescription>
          </DialogHeader>
          <input
            type="text"
            value={renameValue}
            onChange={(event) => setRenameValue(event.target.value)}
            className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 text-sm outline-none focus:border-primary/50"
          />
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setRenameTarget(null);
                setRenameValue('');
              }}
            >
              Cancel
            </Button>
            <Button onClick={() => void submitRename()} disabled={!renameValue.trim()}>
              Rename
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={deleteTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteTarget(null);
            setDeleteRecursive(false);
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Delete {deleteTarget?.name}?</DialogTitle>
            <DialogDescription>
              This action cannot be undone. The operating system permissions of
              the current user apply.
            </DialogDescription>
          </DialogHeader>
          {deleteTarget?.is_dir ? (
            <label className="flex items-center gap-2 text-sm text-muted-foreground">
              <input
                type="checkbox"
                checked={deleteRecursive}
                onChange={(event) => setDeleteRecursive(event.target.checked)}
                className="size-4 rounded border-border bg-surface accent-primary"
              />
              Delete this folder and everything inside it
            </label>
          ) : null}
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDeleteTarget(null);
                setDeleteRecursive(false);
              }}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => void submitDelete()}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={sessionPickerOpen} onOpenChange={setSessionPickerOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Use a session directory</DialogTitle>
            <DialogDescription>
              Jump to the working directory of a live terminal session.
            </DialogDescription>
          </DialogHeader>
          <div className="max-h-72 space-y-2 overflow-y-auto">
            {sessions.length === 0 ? (
              <p className="py-6 text-center text-sm text-muted-foreground">
                No live sessions.
              </p>
            ) : (
              sessions.map((session) => (
                <button
                  key={`${session.kind}-${session.id}`}
                  type="button"
                  onClick={() => void useSessionDirectory(session)}
                  className="flex w-full items-center justify-between rounded-lg border border-border bg-surface/70 px-3 py-2 text-left text-sm transition-colors hover:border-primary/40 hover:bg-surface"
                >
                  <span className="font-mono text-xs">{session.working_directory}</span>
                  <Badge variant="secondary">{session.kind}</Badge>
                </button>
              ))
            )}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function formatSize(size: number): string {
  if (size < 1024) {
    return `${size} B`;
  }

  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KiB`;
  }

  if (size < 1024 * 1024 * 1024) {
    return `${(size / (1024 * 1024)).toFixed(1)} MiB`;
  }

  return `${(size / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
}
