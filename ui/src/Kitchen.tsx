// The kitchen: tickets not yet served, oldest first, by station, each moved on from where it is.
// A dish shows the choices it was ordered with; one voided on its order is struck off, not to be made.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, docPath, refusal, t as tr, isolate } from "@ferrum/client";
import { Button, ErrorState, ListSkeleton, StatusBadge, toast, useShellTitle } from "@ferrum/kit";

interface Ticket {
  id: string;
  station: string;
  table_label: string;
  status: "Queued" | "Preparing" | "Ready" | "Served" | "Voided";
  items?: { dish: string; qty: number; modifiers?: string | null; note?: string | null; voided?: boolean | null }[];
}

/** What a ticket's next step is, from its status. */
/** KitchenTicket.status as stored (packages/restaurant/entities), for `StatusBadge`. */
const TICKET_TONES = { Queued: "neutral", Preparing: "amber", Ready: "green", Served: "neutral", Voided: "neutral" } as const;

export const NEXT: Record<Ticket["status"], { action: string; label: string } | null> = {
  Queued: { action: "start", get label() { return tr("Start it"); } },
  Preparing: { action: "ready", get label() { return tr("Ready"); } },
  Ready: { action: "served", get label() { return tr("Served"); } },
  Served: null,
  Voided: null,
};

export function Kitchen() {
  useShellTitle(tr("Kitchen"));
  const qc = useQueryClient();
  const q = useQuery({
    queryKey: ["restaurant", "kitchen"],
    queryFn: async () => {
      const list = await api.get<{ rows: Ticket[] }>(docPath("KitchenTicket"), { filters: JSON.stringify([["status", "in", ["Queued", "Preparing", "Ready"]]]), sort: "created_at asc", limit: 200 });
      return Promise.all(list.rows.map((t) => api.get<Ticket>(docPath("KitchenTicket", t.id))));
    },
    refetchInterval: 10_000,
  });
  const move = useMutation({
    mutationFn: ({ id, action }: { id: string; action: string }) => api.post(docPath("KitchenTicket", id, "actions", action), {}),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ["restaurant"] }),
    onError: (e) => toast(refusal(e), "error"),
  });
  if (q.isPending) return <ListSkeleton className="np-page" />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  return (
    <div className="np-page">
      <h2>{tr("The kitchen")}</h2>
      <div className="np-columns">
        {q.data.map((t) => {
          const next = NEXT[t.status];
          return (
            <section key={t.id} className="card card-pad stack" aria-label={`${t.station}, ${t.table_label}`}>
              <div className="row" style={{ justifyContent: "space-between" }}>
                <strong>{isolate(t.station)}</strong>
                <StatusBadge value={t.status} tones={TICKET_TONES} />
              </div>
              <div className="faint">{isolate(t.table_label)}</div>
              <ul>
                {(t.items ?? []).map((i, n) => (
                  <li key={n}>
                    {i.voided ? (
                      <span className="faint">
                        <s>
                          {i.qty} × {isolate(i.dish)}
                        </s>{isolate(" ")}
                        {tr("(voided)")}
                      </span>
                    ) : (
                      <>
                        {i.qty} × {isolate(i.dish)} {i.modifiers ? <strong>· {isolate(i.modifiers)}</strong> : null}{" "}
                        {i.note ? <span className="faint">({isolate(i.note)})</span> : null}
                      </>
                    )}
                  </li>
                ))}
              </ul>
              {next && (
                <Button size="sm" loading={move.isPending && move.variables?.id === t.id} onClick={() => move.mutate({ id: t.id, action: next.action })}>
                  {isolate(next.label)}
                </Button>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}
