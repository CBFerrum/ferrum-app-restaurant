// The floor: each table, free or taken with its open order, and the takeaways and deliveries still
// open. A table opens its order; the kitchen has its own page.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, docPath } from "@ferrum/client";
import { Badge, ErrorState, PageLoading, useShellTitle } from "@ferrum/kit";

interface Row {
  id: string;
  [k: string]: unknown;
}

const rows = (entity: string, filters: unknown[], extra: Record<string, string | number> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

/** The open order on each table, by table id. */
export function byTable(orders: Row[]): Map<string, Row> {
  const out = new Map<string, Row>();
  for (const o of orders) if (o.order_type === "Dine In" && typeof o.table === "string") out.set(o.table, o);
  return out;
}

export function Home() {
  useShellTitle("Restaurant");
  const q = useQuery({
    queryKey: ["restaurant", "floor"],
    queryFn: async () => {
      const [tables, orders] = await Promise.all([rows("DiningTable", [], { sort: "table_name asc" }), rows("RestaurantOrder", [["status", "eq", "Open"]])]);
      return { tables, orders };
    },
    refetchInterval: 15_000,
  });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const open = byTable(q.data.orders);
  const away = q.data.orders.filter((o) => o.order_type !== "Dine In");
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>The floor</h2>
        <Link to="/kitchen">The kitchen</Link>
      </header>
      <div className="np-columns">
        {q.data.tables.map((t) => {
          const order = open.get(t.id);
          return (
            <Link key={t.id} className="card card-pad stack" to={order ? `/e/RestaurantOrder/${order.id}` : "/e/RestaurantOrder/new"}>
              <strong>{String(t.table_name)}</strong>
              <span className="faint">{String(t.seats)} seats</span>
              {order ? <Badge tone="amber">{`Taken · ${String(order.number ?? "")}`}</Badge> : <Badge tone="green">Free</Badge>}
            </Link>
          );
        })}
      </div>
      <section className="card card-pad stack" aria-label="Takeaway and delivery">
        <h3>Takeaway and delivery ({away.length})</h3>
        <ul className="stack">
          {away.map((o) => (
            <li key={o.id}>
              <Link to={`/e/RestaurantOrder/${o.id}`}>
                {String(o.number ?? "")} · {String(o.order_type)} {o.delivery_status ? `· ${String(o.delivery_status)}` : ""}
              </Link>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
