import { describe, expect, it } from "vitest";
import { awayLink, byTable, TABLE_TONES, tableLink, toPay } from "./Home";
import { NEXT } from "./Kitchen";

describe("the floor and the kitchen", () => {
  it("finds each table's open order, leaving takeaways and billed orders out", () => {
    const m = byTable([
      { id: "o1", status: "Open", order_type: "Dine In", table: "t1" },
      { id: "o2", status: "Open", order_type: "Takeaway", table: null },
      { id: "o3", status: "Billed", order_type: "Dine In", table: "t2" },
    ]);
    expect(m.get("t1")?.id).toBe("o1");
    expect(m.size).toBe(1);
  });

  it("lists the bills still to be paid", () => {
    const orders = [
      { id: "o1", status: "Open" },
      { id: "o2", status: "Billed" },
      { id: "o3", status: "Settled" },
    ];
    expect(toPay(orders).map((o) => o.id)).toEqual(["o2"]);
  });

  it("moves a ticket on one step at a time", () => {
    expect(NEXT.Queued?.action).toBe("start");
    expect(NEXT.Preparing?.action).toBe("ready");
    expect(NEXT.Ready?.action).toBe("served");
    expect(NEXT.Served).toBeNull();
    expect(NEXT.Voided).toBeNull();
  });

  it("opens a free table's new order for that table and its outlet", () => {
    expect(tableLink({ id: "t1", outlet: "out" }, undefined)).toBe("/e/RestaurantOrder/new?table=t1&outlet=out");
    expect(tableLink({ id: "t1", outlet: "out" }, { id: "o1" })).toBe("/e/RestaurantOrder/o1");
  });

  it("starts a takeaway at the only outlet, and leaves the outlet open when there are several", () => {
    expect(awayLink("Takeaway", [{ id: "out" }])).toBe("/e/RestaurantOrder/new?order_type=Takeaway&outlet=out");
    expect(awayLink("Delivery", [{ id: "a" }, { id: "b" }])).toBe("/e/RestaurantOrder/new?order_type=Delivery");
  });

  it("gives a table's badge its tone from its state, free or taken", () => {
    expect(TABLE_TONES.Free).toBe("green");
    expect(TABLE_TONES.Taken).toBe("amber");
  });
});
