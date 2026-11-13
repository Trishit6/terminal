import type { ReactNode } from 'react';

export function PageShell({
  kicker,
  title,
  description,
  actions,
  aside,
  sidebar,
  children,
}: {
  kicker: string;
  title: ReactNode;
  description: ReactNode;
  actions?: ReactNode;
  aside?: ReactNode;
  sidebar?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="page-shell">
      <header className={aside ? 'page-hero' : 'page-hero page-hero--single'}>
        <div>
          <p className="page-kicker">{kicker}</p>
          <h1>{title}</h1>
          <p className="page-lede">{description}</p>
          {actions ? <div className="page-actions">{actions}</div> : null}
        </div>
        {aside ? <aside className="page-aside">{aside}</aside> : null}
      </header>
      <div className={sidebar ? 'page-body page-body--sidebar' : 'page-body'}>
        {sidebar ? <aside className="page-sidebar">{sidebar}</aside> : null}
        <div className="page-body__main">{children}</div>
      </div>
    </div>
  );
}

export function PageAsideRow({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="page-aside__row">
      <span>{label}</span>
      <b>{value}</b>
    </div>
  );
}
