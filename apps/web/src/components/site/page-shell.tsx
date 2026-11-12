import type { ReactNode } from 'react';

export function PageShell({
  kicker,
  title,
  description,
  actions,
  aside,
  children,
}: {
  kicker: string;
  title: ReactNode;
  description: ReactNode;
  actions?: ReactNode;
  aside?: ReactNode;
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
      <div className="page-body">{children}</div>
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
