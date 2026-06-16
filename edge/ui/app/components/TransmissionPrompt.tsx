import type { Transmission } from "../../store/types";

interface Props {
  transmission: Transmission;
  pending: boolean;
  onAnswer: (id: string, response: string) => void;
}

export function TransmissionPrompt({ transmission, pending, onAnswer }: Props) {
  const t = transmission;
  return (
    <section className="prompt-dock" role="alertdialog" aria-label="Permission requested">
      <span className="prompt-kicker">
        <span className="dot" style={{ width: 7, height: 7, borderRadius: 99, background: "var(--alert)" }} />
        {t.kind === "permission" ? "Permission requested" : "Question"}
      </span>
      <p className="prompt-q">{t.prompt}</p>
      <div className="prompt-actions">
        {t.options.map((opt, i) => (
          <button
            key={opt.id}
            className="btn"
            data-variant={i === 0 ? "primary" : undefined}
            disabled={pending}
            onClick={() => onAnswer(t.id, opt.id)}
          >
            {opt.label}
          </button>
        ))}
      </div>
    </section>
  );
}
