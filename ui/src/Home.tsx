// The floor: each table, free or taken with its open order, the takeaways and deliveries still
// open, and the bills still to be paid. A table opens its order, or a new one for that table; a bill
// to pay opens its order, where payment is taken; the kitchen has its own page.

import { useQuery } from "@tanstack/react-query";
import { api, docPath, t as tr, isolate } from "@ferrum/client";
import { ButtonLink, HomeSkeleton, LoadFailed, PageHead, Queue, QueueGrid, QueueItem, StatePanel, StatusBadge, Tile, TileGrid, type Tone } from "@ferrum/kit";

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
  for (const o of orders) if (o.status === "Open" && o.order_type === "Dine In" && typeof o.table === "string") out.set(o.table, o);
  return out;
}

/** A table on the floor is free or taken (it has an open order); the badge's tone comes from that state id, not its words. */
export const TABLE_TONES: Readonly<Record<"Free" | "Taken", Tone>> = { Free: "green", Taken: "amber" };

/** The orders billed and not yet paid, oldest first. */
export function toPay(orders: Row[]): Row[] {
  return orders.filter((o) => o.status === "Billed");
}

/** The most a queue lists before "and N more" opens the orders. */
const LIMIT = 5;

export function Home() {
  const q = useQuery({
    queryKey: ["restaurant", "floor"],
    queryFn: async () => {
      const [tables, orders, outlets] = await Promise.all([
        rows("DiningTable", [], { sort: "table_name asc" }),
        rows("RestaurantOrder", [["status", "in", ["Open", "Billed"]]], { sort: "created_at asc" }),
        rows("Outlet", [], { fields: "id,outlet_name" }),
      ]);
      return { tables, orders, outlets };
    },
    refetchInterval: 15_000,
  });
  const title = tr("The floor");
  if (q.isPending) return <><PageHead title={title} /><HomeSkeleton queues={2} /></>;
  if (q.isError) return <><PageHead title={title} /><LoadFailed error={q.error} onRetry={() => void q.refetch()} /></>;
  const open = byTable(q.data.orders);
  const away = q.data.orders.filter((o) => o.status === "Open" && o.order_type !== "Dine In");
  const unpaid = toPay(q.data.orders);
  const tableName = new Map(q.data.tables.map((t) => [t.id, String(t.table_name)]));
  if (!q.data.tables.length)
    return (
      <>
        <PageHead title={title} />
        <StatePanel
          kind="empty"
          title={tr("No tables yet")}
          action={
            q.data.outlets.length ? (
              <ButtonLink to="/e/DiningTable/new" variant="primary" icon="plus">{tr("Add a table")}</ButtonLink>
            ) : (
              <ButtonLink to="/e/Outlet/new" variant="primary" icon="plus">{tr("Add an outlet")}</ButtonLink>
            )
          }
        >
          {tr("Add an outlet with its company, a walk-in customer for bills that name nobody and its active menu, then its tables, and the floor shows here.")}
        </StatePanel>
      </>
    );
  const order = (o: Row) => String(o.number ?? "") || tr("Order");
  return (
    <>
      <PageHead
        title={title}
        description={tr("{taken} of {total} tables taken", { taken: open.size, total: q.data.tables.length })}
        actions={
          <>
            <ButtonLink to={awayLink("Takeaway", q.data.outlets)} variant="primary" icon="plus">{tr("New takeaway")}</ButtonLink>
            <ButtonLink to={awayLink("Delivery", q.data.outlets)} icon="plus">{tr("New delivery")}</ButtonLink>
            <ButtonLink to="/kitchen">{tr("The kitchen")}</ButtonLink>
          </>
        }
      />
      <TileGrid label={title}>
        {q.data.tables.map((t) => {
          const taken = open.get(t.id);
          return (
            <Tile
              key={t.id}
              to={tableLink(t, taken)}
              title={isolate(String(t.table_name))}
              meta={<span>{tr("{value} seats", { value: String(t.seats) })}</span>}
              status={taken ? <StatusBadge value="Taken" tones={TABLE_TONES} label={tr("Taken · {value}", { value: String(taken.number ?? "") })} /> : <StatusBadge value="Free" tones={TABLE_TONES} label={tr("Free")} />}
            />
          );
        })}
      </TileGrid>
      <QueueGrid className="mt-6">
        <Queue title={tr("Bills to pay")} count={unpaid.length} tone={unpaid.length ? "attention" : "default"} empty={tr("Every bill is paid.")} more={{ to: "/e/RestaurantOrder", count: unpaid.length - LIMIT }}>
          {unpaid.slice(0, LIMIT).map((o) => (
            <QueueItem
              key={o.id}
              to={`/e/RestaurantOrder/${o.id}`}
              title={isolate(order(o))}
              meta={isolate(tableName.get(String(o.table)) ?? String(o.order_type ?? ""))}
              trailing={<span className="text-sm font-medium tabular-nums">{isolate(String(o.total ?? ""))}</span>}
            />
          ))}
        </Queue>
        <Queue title={tr("Takeaway and delivery")} count={away.length} more={{ to: "/e/RestaurantOrder", count: away.length - LIMIT }}>
          {away.slice(0, LIMIT).map((o) => (
            <QueueItem
              key={o.id}
              to={`/e/RestaurantOrder/${o.id}`}
              title={isolate(order(o))}
              meta={isolate(String(o.order_type))}
              trailing={o.delivery_status ? <StatusBadge value={o.delivery_status} /> : undefined}
            />
          ))}
        </Queue>
      </QueueGrid>
    </>
  );
}
