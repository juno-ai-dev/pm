import type { ButtonHTMLAttributes, ReactNode } from 'react'

export function Button({ children, variant = 'primary', ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { children: ReactNode; variant?: 'primary' | 'secondary' | 'ghost' }) {
  return <button type="button" className={`button button--${variant}`} {...props}>{children}</button>
}

export function StatePanel({ kind, title, children, action }: { kind: 'loading' | 'error' | 'empty' | 'wallet'; title: string; children: ReactNode; action?: ReactNode }) {
  return (
    <section className={`state-panel state-panel--${kind}`} aria-live={kind === 'loading' ? 'polite' : undefined} aria-busy={kind === 'loading' || undefined}>
      <span className="state-panel__signal" aria-hidden="true" />
      <p className="eyebrow">{kind}</p>
      <h2>{title}</h2>
      <div className="state-panel__copy">{children}</div>
      {action && <div className="state-panel__action">{action}</div>}
    </section>
  )
}
