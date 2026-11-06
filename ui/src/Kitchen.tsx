// The kitchen: tickets not yet served, oldest first, by station, each moved on from where it is.
// A dish voided on its order is struck off, not to be made.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, docPath, refusal } from "@ferrum/client";
import { Badge, Button, ErrorState, PageLoading, toast, useShellTitle } from "@ferrum/kit";

interface Ticket {
  id: string;
  station: string;
  table_label: string;
  status: "Queued" | "Preparing" | "Ready" | "Served" | "Voided";
  items?: { dish: string; qty: number; note?: string | null; voided?: boolean | null }[];
}

/** What a ticket's next step is, from its status. */
export const NEXT: Record<Ticket["status"], { action: string; label: string } | null> = {
  Queued: { action: "start", label: "Start it" },
  Preparing: { action: "ready", label: "Ready" },
  Ready: { action: "served", label: "Served" },
  Served: null,
  Voided: null,
};

export function Kitchen() {
  useShellTitle("Kitchen");
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
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  return (
    <div className="np-page">
      <h2>The kitchen</h2>
      <div className="np-columns">
        {q.data.map((t) => {
          const next = NEXT[t.status];
          return (
            <section key={t.id} className="card card-pad stack" aria-label={`${t.station}, ${t.table_label}`}>
              <div className="row" style={{ justifyContent: "space-between" }}>
                <strong>{t.station}</strong>
                <Badge tone={t.status === "Ready" ? "green" : t.status === "Preparing" ? "amber" : "neutral"}>{t.status}</Badge>
              </div>
              <div className="faint">{t.table_label}</div>
              <ul>
                {(t.items ?? []).map((i, n) => (
                  <li key={n}>
                    {i.voided ? (
                      <span className="faint">
                        <s>
                          {i.qty} × {i.dish}
                        </s>{" "}
                        (voided)
                      </span>
                    ) : (
                      <>
                        {i.qty} × {i.dish} {i.note ? <span className="faint">({i.note})</span> : null}
                      </>
                    )}
                  </li>
                ))}
              </ul>
              {next && (
                <Button size="sm" loading={move.isPending && move.variables?.id === t.id} onClick={() => move.mutate({ id: t.id, action: next.action })}>
                  {next.label}
                </Button>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}
