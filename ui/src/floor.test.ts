import { describe, expect, it } from "vitest";
import { byTable } from "./Home";
import { NEXT } from "./Kitchen";

describe("the floor and the kitchen", () => {
  it("finds each table's open order, leaving takeaways out", () => {
    const m = byTable([
      { id: "o1", order_type: "Dine In", table: "t1" },
      { id: "o2", order_type: "Takeaway", table: null },
    ]);
    expect(m.get("t1")?.id).toBe("o1");
    expect(m.size).toBe(1);
  });

  it("moves a ticket on one step at a time", () => {
    expect(NEXT.Queued?.action).toBe("start");
    expect(NEXT.Preparing?.action).toBe("ready");
    expect(NEXT.Ready?.action).toBe("served");
    expect(NEXT.Served).toBeNull();
  });
});
