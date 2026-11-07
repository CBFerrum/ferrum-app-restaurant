// The floor: each table, free or taken with its open order, and the takeaways and deliveries still
// open. A table opens its order, or a new one for that table; the kitchen has its own page.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, docPath, t as tr, isolate } from "@ferrum/client";
import { Badge, EmptyState, ErrorState, PageLoading, useShellTitle } from "@ferrum/kit";

interface Row {
  id: string;
  [k: string]: unknown;
}

const rows = (entity: string, filters: unknown[], extra: Record<string, string | number> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

/** Where a table on the floor leads: its open order, or a new order for that table. */
export function tableLink(table: Row, order: Row | undefined): string {
  if (order) return `/e/RestaurantOrder/${order.id}`;
  const q = new URLSearchParams({ table: table.id });
  if (typeof table.outlet === "string") q.set("outlet", table.outlet);
  return `/e/RestaurantOrder/new?${q}`;
}

/** A new takeaway or delivery, at the outlet when there is only one. */
export function awayLink(kind: "Takeaway" | "Delivery", outlets: Row[]): string {
  const q = new URLSearchParams({ order_type: kind });
  if (outlets.length === 1) q.set("outlet", outlets[0].id);
  return `/e/RestaurantOrder/new?${q}`;
}

/** The open order on each table, by table id. */
export function byTable(orders: Row[]): Map<string, Row> {
  const out = new Map<string, Row>();
  for (const o of orders) if (o.order_type === "Dine In" && typeof o.table === "string") out.set(o.table, o);
  return out;
}

export function Home() {
  useShellTitle(tr("Restaurant"));
  const q = useQuery({
    queryKey: ["restaurant", "floor"],
    queryFn: async () => {
      const [tables, orders, outlets] = await Promise.all([
        rows("DiningTable", [], { sort: "table_name asc" }),
        rows("RestaurantOrder", [["status", "eq", "Open"]]),
        rows("Outlet", [], { fields: "id,outlet_name" }),
      ]);
      return { tables, orders, outlets };
    },
    refetchInterval: 15_000,
  });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const open = byTable(q.data.orders);
  const away = q.data.orders.filter((o) => o.order_type !== "Dine In");
  if (!q.data.tables.length)
    return (
      <div className="np-page">
        <EmptyState title={tr("No tables yet")} icon="list">
          <p className="muted">
            {tr("Add an outlet with its company, a walk-in customer for bills that name nobody and its active menu, then its tables, and the floor shows here.")}{isolate(" ")}
            {q.data.outlets.length ? <Link to="/e/DiningTable/new">{tr("Add a table")}</Link> : <Link to="/e/Outlet/new">{tr("Add an outlet")}</Link>}
          </p>
        </EmptyState>
      </div>
    );
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>{tr("The floor")}</h2>
        <span className="row">
          <Link to={awayLink("Takeaway", q.data.outlets)}>{tr("New takeaway")}</Link>
          <Link to={awayLink("Delivery", q.data.outlets)}>{tr("New delivery")}</Link>
          <Link to="/kitchen">{tr("The kitchen")}</Link>
        </span>
      </header>
      <div className="np-columns">
        {q.data.tables.map((t) => {
          const order = open.get(t.id);
          return (
            <Link key={t.id} className="card card-pad stack" to={tableLink(t, order)}>
              <strong>{isolate(String(t.table_name))}</strong>
              <span className="faint">{tr("{value} seats", { value: String(t.seats) })}</span>
              {order ? <Badge tone="amber">{tr("Taken · {value}", { value: String(order.number ?? "") })}</Badge> : <Badge tone="green">{tr("Free")}</Badge>}
            </Link>
          );
        })}
      </div>
      <section className="card card-pad stack" aria-label={tr("Takeaway and delivery")}>
        <h3>{tr("Takeaway and delivery ({length})", { length: away.length })}</h3>
        <ul className="stack">
          {away.map((o) => (
            <li key={o.id}>
              <Link to={`/e/RestaurantOrder/${o.id}`}>
                {isolate(String(o.number ?? ""))} · {isolate(String(o.order_type))} {o.delivery_status ? `· ${String(o.delivery_status)}` : ""}
              </Link>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
