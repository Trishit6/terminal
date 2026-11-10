import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Check,
  Command as CommandIcon,
  Cpu,
  FolderOpen,
  Monitor,
  RefreshCw,
  Search,
  Terminal,
  X,
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
  desktopCommandCatalog,
  type CommandCatalogEntry,
  type CommandCategory,
  type CommandKind,
} from '@/lib/desktop';
import { cn } from '@/lib/utils';

const CATEGORY_LABELS: Record<CommandCategory, string> = {
  core_shell: 'Core shell',
  filesystem: 'Filesystem',
  text_processing: 'Text processing',
  system: 'System',
  development: 'Development',
  networking: 'Networking',
};

const CATEGORY_ICONS: Record<CommandCategory, typeof Cpu> = {
  core_shell: Terminal,
  filesystem: FolderOpen,
  text_processing: CommandIcon,
  system: Monitor,
  development: Cpu,
  networking: Monitor,
};

export function DesktopCommandExplorerPage() {
  const [catalog, setCatalog] = useState<CommandCatalogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState<CommandCategory | 'all'>('all');
  const [kind, setKind] = useState<CommandKind | 'all'>('all');
  const [availableOnly, setAvailableOnly] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const next = await desktopCommandCatalog();
      setCatalog(next);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const categories = useMemo(() => {
    const values = new Set<CommandCategory>();
    catalog.forEach((entry) => values.add(entry.category));
    return Array.from(values).sort();
  }, [catalog]);

  const entries = useMemo(() => {
    const needle = query.trim().toLowerCase();

    return catalog.filter((entry) => {
      if (category !== 'all' && entry.category !== category) {
        return false;
      }

      if (kind !== 'all' && entry.kind !== kind) {
        return false;
      }

      if (availableOnly && !entry.available) {
        return false;
      }

      if (!needle) {
        return true;
      }

      return (
        entry.name.toLowerCase().includes(needle) ||
        entry.description.toLowerCase().includes(needle) ||
        entry.usage.some((usage) => usage.toLowerCase().includes(needle))
      );
    });
  }, [availableOnly, catalog, category, kind, query]);

  return (
    <div className="mx-auto w-full max-w-[1400px] px-6 py-8 lg:px-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.22em] text-primary">
            Command explorer
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Available commands
          </h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            Built-ins come from the native registry. External commands are
            resolved through the real PATH on this machine.
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

      <div className="mt-6 flex flex-wrap items-center gap-3">
        <div className="relative flex-1 basis-64">
          <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search commands, descriptions, or usage…"
            className="h-10 w-full rounded-lg border border-border bg-surface/70 pl-9 pr-9 text-sm outline-none transition-colors placeholder:text-muted-foreground focus:border-primary/50"
          />
          {query ? (
            <button
              type="button"
              onClick={() => setQuery('')}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground"
              aria-label="Clear search"
            >
              <X className="size-4" />
            </button>
          ) : null}
        </div>

        <select
          value={category}
          onChange={(event) => setCategory(event.target.value as CommandCategory | 'all')}
          className="h-10 rounded-lg border border-border bg-surface/70 px-3 text-sm outline-none focus:border-primary/50"
          aria-label="Filter by category"
        >
          <option value="all">All categories</option>
          {categories.map((value) => (
            <option key={value} value={value}>
              {CATEGORY_LABELS[value]}
            </option>
          ))}
        </select>

        <select
          value={kind}
          onChange={(event) => setKind(event.target.value as CommandKind | 'all')}
          className="h-10 rounded-lg border border-border bg-surface/70 px-3 text-sm outline-none focus:border-primary/50"
          aria-label="Filter by kind"
        >
          <option value="all">All kinds</option>
          <option value="builtin">Built-in</option>
          <option value="external">External</option>
        </select>

        <label className="inline-flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
          <input
            type="checkbox"
            checked={availableOnly}
            onChange={(event) => setAvailableOnly(event.target.checked)}
            className="size-4 rounded border-border bg-surface accent-primary"
          />
          Available only
        </label>
      </div>

      <div className="mt-6 grid gap-3 lg:grid-cols-2">
        {entries.map((entry) => {
          const Icon = CATEGORY_ICONS[entry.category];

          return (
            <Card key={`${entry.kind}-${entry.name}`} className="overflow-hidden">
              <CardHeader className="gap-2">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-center gap-2">
                    <div className="grid size-8 place-items-center rounded-lg border border-border bg-background/70 text-primary">
                      <Icon className="size-4" />
                    </div>
                    <CardTitle className="font-mono text-base">{entry.name}</CardTitle>
                  </div>
                  <div className="flex flex-wrap items-center gap-1.5">
                    <Badge variant={entry.available ? 'success' : 'secondary'}>
                      {entry.available ? (
                        <Check className="mr-1 size-3" />
                      ) : null}
                      {entry.available ? 'available' : 'missing'}
                    </Badge>
                    <Badge variant={entry.kind === 'builtin' ? 'default' : 'outline'}>
                      {entry.kind}
                    </Badge>
                  </div>
                </div>
                <CardDescription>{entry.description}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-2">
                <div className="text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
                  {CATEGORY_LABELS[entry.category]}
                </div>
                {entry.usage.length > 0 ? (
                  <div className="space-y-1">
                    {entry.usage.map((usage) => (
                      <code
                        key={usage}
                        className="block truncate rounded-md border border-border bg-background/70 px-2 py-1 font-mono text-xs"
                      >
                        {usage}
                      </code>
                    ))}
                  </div>
                ) : null}
                <div className="flex flex-wrap gap-1.5">
                  {entry.platforms.map((platform) => (
                    <span
                      key={platform}
                      className={cn(
                        'rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground',
                      )}
                    >
                      {platform}
                    </span>
                  ))}
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>

      {!loading && entries.length === 0 ? (
        <div className="mt-10 rounded-xl border border-border bg-surface/50 py-12 text-center text-sm text-muted-foreground">
          No commands match the current filters.
        </div>
      ) : null}
    </div>
  );
}
