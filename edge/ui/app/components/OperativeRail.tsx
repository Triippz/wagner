import type { Operative } from "../../store/types";

function glyph(name: string): string {
  const parts = name.trim().split(/[\s_-]+/).filter(Boolean);
  if (parts.length >= 2) return (parts[0]![0]! + parts[1]![0]!).toUpperCase();
  return name.slice(0, 2).toUpperCase();
}

interface Props {
  operatives: Operative[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function OperativeRail({ operatives, selectedId, onSelect }: Props) {
  const sorted = [...operatives].sort((a, b) => a.name.localeCompare(b.name));
  return (
    <aside className="rail">
      <div className="rail-head">
        <h2>Roster</h2>
        <span className="count">{operatives.length}</span>
      </div>
      <div className="op-list">
        {sorted.map((op, i) => {
          const sub = op.bubble?.trim() || op.activity;
          return (
            <button
              key={op.id}
              className="op"
              data-faction={op.faction}
              data-selected={op.id === selectedId}
              style={{ animationDelay: `${Math.min(i * 24, 240)}ms` }}
              onClick={() => onSelect(op.id)}
            >
              <span className="op-glyph">{glyph(op.name)}</span>
              <span className="op-main">
                <div className="op-name">{op.name}</div>
                <div className="op-sub">{sub}</div>
              </span>
              <span className="op-state" data-s={op.state}>
                {op.state}
              </span>
            </button>
          );
        })}
        {operatives.length === 0 && (
          <p className="empty-note" style={{ padding: "24px 12px" }}>
            No operatives on the floor yet.
          </p>
        )}
      </div>
    </aside>
  );
}
