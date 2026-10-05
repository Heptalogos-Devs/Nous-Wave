import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { randomUUID } from "node:crypto";

export interface ExecutionLimits {
  providerCalls: number;
  newEmbeddingItems: number;
  rerankCalls: number;
  newServingGenerations: number;
  newArtifactBytes: number;
  runtimeSeconds: number;
}
export type WorkReservation = Partial<Omit<ExecutionLimits, "runtimeSeconds">>;
interface Ledger {
  identity: string;
  limits: ExecutionLimits;
  startedAt: number;
  used: Required<WorkReservation>;
  stopReason: string | null;
}
export class ResearchBudgetStop extends Error {}

/** Reserve work before dispatch, including retries. One serial run owns this ledger. */
export class ExecutionBudget {
  private queue: Promise<void> = Promise.resolve();
  private readonly controller = new AbortController();
  private timer?: NodeJS.Timeout;
  private constructor(
    private readonly path: string,
    private readonly ledger: Ledger,
  ) {}
  static async open(path: string, identity: string, limits: ExecutionLimits) {
    for (const [key, value] of Object.entries(limits)) {
      if (
        !Number.isSafeInteger(value) ||
        value < 0 ||
        (key === "runtimeSeconds" && value === 0)
      )
        throw new Error(`Invalid execution limit: ${key}`);
    }
    let ledger: Ledger = {
      identity,
      limits,
      startedAt: Date.now(),
      stopReason: null,
      used: {
        providerCalls: 0,
        newEmbeddingItems: 0,
        rerankCalls: 0,
        newServingGenerations: 0,
        newArtifactBytes: 0,
      },
    };
    try {
      const saved = JSON.parse(await readFile(path, "utf8")) as Ledger;
      if (
        saved.identity !== identity ||
        JSON.stringify(saved.limits) !== JSON.stringify(limits) ||
        !Number.isSafeInteger(saved.startedAt) ||
        !saved.used ||
        (saved.stopReason !== null && typeof saved.stopReason !== "string") ||
        Object.keys(ledger.used).some(
          (key) =>
            !Number.isSafeInteger(saved.used[key as keyof WorkReservation]) ||
            saved.used[key as keyof WorkReservation] < 0,
        )
      )
        throw new Error("Execution ledger does not match this run");
      ledger = saved;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
    const budget = new ExecutionBudget(path, ledger);
    await budget.persist();
    const remaining =
      limits.runtimeSeconds * 1000 - (Date.now() - ledger.startedAt);
    if (ledger.stopReason)
      budget.controller.abort(new ResearchBudgetStop(ledger.stopReason));
    else if (remaining <= 0) await budget.stop("runtimeSeconds exhausted");
    else
      budget.timer = setTimeout(() => {
        budget.controller.abort(
          new ResearchBudgetStop("runtimeSeconds exhausted"),
        );
        void budget.stop("runtimeSeconds exhausted").catch(() => {});
      }, remaining);
    return budget;
  }
  get signal() {
    return this.controller.signal;
  }
  snapshot() {
    return structuredClone({
      ...this.ledger,
      elapsedSeconds: (Date.now() - this.ledger.startedAt) / 1000,
    });
  }
  async reserve(work: WorkReservation) {
    return this.serial(async () => {
      if (this.ledger.stopReason || this.signal.aborted)
        throw new ResearchBudgetStop(
          this.ledger.stopReason ?? "runtimeSeconds exhausted",
        );
      if (
        Date.now() - this.ledger.startedAt >=
        this.ledger.limits.runtimeSeconds * 1000
      ) {
        await this.stopInQueue("runtimeSeconds exhausted");
        throw new ResearchBudgetStop("runtimeSeconds exhausted");
      }
      for (const [key, value] of Object.entries(work)) {
        if (
          !(key in this.ledger.used) ||
          !Number.isSafeInteger(value) ||
          value < 0
        )
          throw new Error(`Invalid work reservation: ${key}`);
        const counter = key as keyof WorkReservation;
        if (this.ledger.used[counter] + value > this.ledger.limits[counter]) {
          await this.stopInQueue(`${key} exhausted`);
          throw new ResearchBudgetStop(`${key} exhausted`);
        }
      }
      for (const [key, value] of Object.entries(work))
        this.ledger.used[key as keyof WorkReservation] += value;
      await this.persist();
    });
  }
  async observeArtifactBytes(delta: number) {
    if (!Number.isSafeInteger(delta) || delta < 0)
      throw new Error("Invalid artifact delta");
    await this.serial(async () => {
      this.ledger.used.newArtifactBytes = delta;
      if (delta >= this.ledger.limits.newArtifactBytes)
        await this.stopInQueue("newArtifactBytes exhausted");
      else await this.persist();
    });
  }
  async observeServingGenerations(delta: number) {
    if (!Number.isSafeInteger(delta) || delta < 0)
      throw new Error("Invalid generation delta");
    await this.serial(async () => {
      this.ledger.used.newServingGenerations = delta;
      if (delta > this.ledger.limits.newServingGenerations)
        await this.stopInQueue("newServingGenerations exhausted");
      else await this.persist();
    });
  }
  async stop(reason: string) {
    await this.serial(() => this.stopInQueue(reason));
  }
  async close() {
    clearTimeout(this.timer);
    await this.queue;
    await this.persist();
  }
  private async stopInQueue(reason: string) {
    this.ledger.stopReason ??= reason;
    this.controller.abort(new ResearchBudgetStop(this.ledger.stopReason));
    await this.persist();
  }
  private serial<T>(operation: () => Promise<T>): Promise<T> {
    const result = this.queue.then(operation);
    this.queue = result.then(() => {}).catch(() => {});
    return result;
  }
  private async persist() {
    await mkdir(dirname(this.path), { recursive: true });
    const temporary = `${this.path}.${randomUUID()}.tmp`;
    await writeFile(temporary, JSON.stringify(this.ledger, null, 2) + "\n", {
      flag: "wx",
      mode: 0o600,
    });
    await rename(temporary, this.path);
  }
}
