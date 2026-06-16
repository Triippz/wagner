// T000c — Ephemeral node-discovery registry (hub storage infra).
//
// Per armed operator: the iroh NodeId + signaling ticket a verified OWNING peer
// resolves to attach. Ephemeral and in-memory by design — it carries no
// run-bearing content (F-1), armed hosts re-register, and "tens of operators"
// scale (plan §Scale) needs nothing durable. Ownership is enforced on resolve.

export interface Registration {
  operatorId: string;
  nodeId: string;
  ticket: string;
  expiresAt: number;
}

export interface RegisterInput {
  operatorId: string;
  nodeId: string;
  ticket: string;
  ttlMs: number;
}

export interface ResolveInput {
  /** The operator whose host is being resolved. */
  operatorId: string;
  /** The verified operator making the request (ownership check). */
  requesterId: string;
}

export interface RegistryOptions {
  nowMs?: () => number;
}

export class DiscoveryRegistry {
  #entries = new Map<string, Registration>();
  #now: () => number;

  constructor(opts: RegistryOptions = {}) {
    this.#now = opts.nowMs ?? (() => Date.now());
  }

  /** Arm (or re-arm) — first-write-wins is irrelevant; this replaces in place. */
  register(input: RegisterInput): Registration {
    const reg: Registration = {
      operatorId: input.operatorId,
      nodeId: input.nodeId,
      ticket: input.ticket,
      expiresAt: this.#now() + input.ttlMs,
    };
    this.#entries.set(input.operatorId, reg);
    return reg;
  }

  /** Owner-only resolve; null if not armed, expired, or requester ≠ owner. */
  resolve(input: ResolveInput): Registration | null {
    if (input.requesterId !== input.operatorId) return null;
    const reg = this.#entries.get(input.operatorId);
    if (!reg) return null;
    if (this.#now() >= reg.expiresAt) {
      this.#entries.delete(input.operatorId);
      return null;
    }
    return reg;
  }

  disarm(operatorId: string): void {
    this.#entries.delete(operatorId);
  }

  /** Count of live (un-swept) entries — test/observability aid. */
  size(): number {
    return this.#entries.size;
  }
}
