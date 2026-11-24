// A new note made at a spot of the board: what the store remembers to take it away again
// (`placed`, `added`) is kept only while its title waits, and let go after.
import { beforeAll, describe, expect, it } from "vitest";
import { testStore } from "./testStore";

describe("a new note at a spot", () => {
  let s: Awaited<ReturnType<typeof testStore>>;
  beforeAll(async () => {
    s = await testStore();
  });
  const placed = () => (s.store as unknown as { placed: Map<number, unknown> }).placed;
  const added = () => (s.store as unknown as { added: Map<number, unknown> }).added;
  const order = () => [...s.store.notes].sort((a, b) => a.sort - b.sort).map((n) => n.id);
  const board = (slow = 0) => {
    const { store } = s;
    store.board = {
      point: (p) => p,
      around: () => ({ above: null, below: null }),
      place: async (id) => {
        const ids = order().filter((x) => x !== id);
        if (slow) await new Promise((r) => setTimeout(r, slow));
        await store.saveBoardSteps("all", 2, [ids[0], id, ...ids.slice(1)].map((x, i) => ({ id: x, c: i % 2 })));
      },
    };
  };

  it("a template note at a spot (not fresh) is placed but not remembered", async () => {
    board();
    const tpl = { id: -1, name: "T", palette: "soft", color: "Butter", paper: "plain", lines: [{ kind: "task", text: "a" }] } as never;
    const n = await s.store.fromTemplate(tpl, "", { at: { x: 1, y: 1 } });
    expect(placed().size).toBe(0);
    expect(added().size).toBe(0);
    const steps = (s.store.settings.board_steps as Record<string, { id: number }[]>)["all:2"].map((x) => x.id);
    expect(steps).toContain(n.id);
    await s.store.history.undo();
    expect(s.store.notes.some((x) => x.id === n.id)).toBe(false);
    await s.store.history.redo();
    expect(s.store.notes.some((x) => x.id === n.id)).toBe(true);
    expect(placed().size).toBe(0);
  });

  it("fresh, Ctrl+Z while the title waits: forgotten; redo, undo twice: nothing left", async () => {
    board();
    const n = await s.store.fromTemplate(null, "", { at: { x: 1, y: 1 }, fresh: true });
    expect(placed().has(n.id)).toBe(true);
    await s.store.history.undo();
    expect(placed().has(n.id)).toBe(false);
    expect(added().has(n.id)).toBe(false);
    expect(s.store.fresh).toBe(null);
    // The card's blur after the undo: dropNew does nothing.
    expect(await s.store.dropNew(n.id)).toBe(false);
    await s.store.history.redo();
    expect(s.store.notes.some((x) => x.id === n.id)).toBe(true);
    expect(placed().has(n.id)).toBe(false);
    await s.store.history.undo();
    await s.store.history.undo();
    expect(placed().has(n.id)).toBe(false);
    await s.store.history.redo();
    await s.store.history.redo();
  });

  it("fresh, Esc while the place is still saving: the board as it was", async () => {
    board(80);
    const before = order();
    const stepsBefore = structuredClone(s.store.settings.board_steps ?? {});
    const n = await s.store.fromTemplate(null, "", { at: { x: 1, y: 1 }, fresh: true });
    expect(await s.store.dropNew(n.id)).toBe(true);
    expect(placed().size).toBe(0);
    expect(s.store.settings.board_steps ?? {}).toEqual(stepsBefore);
    expect(order()).toEqual(before);
  });

  it("named: nothing is kept for it", async () => {
    board();
    const n = await s.store.fromTemplate(null, "", { at: { x: 1, y: 1 }, fresh: true });
    await s.store.nameNew(n, "Kept");
    expect(placed().has(n.id)).toBe(false);
    expect(added().size).toBe(0);
    s.store.board = null;
  });
});
