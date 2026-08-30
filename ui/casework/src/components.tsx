import type { ReactNode } from "react";

export function Exact({ children, wrap = false }: { children: ReactNode; wrap?: boolean }) {
  return <code className={wrap ? "exact wrap" : "exact"}>{children}</code>;
}

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="field">
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

export function StringList({ values, empty = "None recorded" }: { values: string[]; empty?: string }) {
  if (values.length === 0) return <p className="empty">{empty}</p>;
  return (
    <ul className="string-list">
      {values.map((value, index) => <li key={`${index}-${value}`}><Exact wrap>{value}</Exact></li>)}
    </ul>
  );
}

export function Section({ title, children, className = "", id, tabIndex }: { title: string; children: ReactNode; className?: string; id?: string; tabIndex?: number }) {
  return (
    <section className={["case-section", className].filter(Boolean).join(" ")} id={id} tabIndex={tabIndex}>
      <h2>{title}</h2>
      {children}
    </section>
  );
}

export function DefinitionGrid({ children }: { children: ReactNode }) {
  return <dl className="definition-grid">{children}</dl>;
}
