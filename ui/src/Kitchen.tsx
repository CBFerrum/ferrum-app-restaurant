// The kitchen: tickets not yet served, oldest first, by station, each moved on from where it is.
// A dish shows the choices it was ordered with; one voided on its order is struck off, not to be made.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, docPath, refusal, t as tr, isolate } from "@ferrum/client";
import { Button, HomeSkeleton, LoadFailed, PageHead, PageStack, StatePanel, StatusBadge, toast, type Tone } from "@ferrum/kit";

interface Ticket {
  id: string;
  station: string;
  table_label: string;
  status: "Queued" | "Preparing" | "Ready" | "Served" | "Voided";
  items?: { dish: string; qty: number; modifiers?: string | null; note?: string | null; voided?: boolean | null }[];
}

/** What a ticket's next step is, from its status. */
/** KitchenTicket.status as stored (packages/restaurant/entities), for `StatusBadge`. */
const TICKET_TONES: Readonly<Record<Ticket["status"], Tone>> = { Queued: "neutral", Preparing: "amber", Ready: "green", Served: "neutral", Voided: "neutral" };

export const NEXT: Record<Ticket["status"], { action: string; label: string } | null> = {
  Queued: { action: "start", get label() { return tr("Start it"); } },
  Preparing: { action: "ready", get label() { return tr("Ready"); } },
  Ready: { action: "served", get label() { return tr("Served"); } },
  Served: null,
  Voided: null,
};

export function Kitchen() {
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
  const title = tr("The kitchen");
  const back = { to: "/", label: tr("The floor") };
  if (q.isPending) return <PageStack><PageHead title={title} back={back} /><HomeSkeleton queues={3} /></PageStack>;
  if (q.isError) return <PageStack><PageHead title={title} back={back} /><LoadFailed error={q.error} onRetry={() => void q.refetch()} /></PageStack>;
  if (!q.data.length)
    return (
      <PageStack>
        <PageHead title={title} back={back} />
        <StatePanel kind="empty" title={tr("No tickets waiting")}>{tr("Tickets appear here as soon as the floor sends dishes to a station.")}</StatePanel>
      </PageStack>
    );
  // A ticket moves on only when the server has taken the step: the list is fetched again after it, never changed in place.
  return (
    <PageStack>
      <PageHead title={title} back={back} />
      <div className="kitchen-grid">
        {q.data.map((t) => {
          const next = NEXT[t.status];
          return (
            <section key={t.id} className="card card-pad stack kitchen-ticket" aria-label={`${t.station}, ${t.table_label}`}>
              <div className="row" style={{ justifyContent: "space-between" }}>
                <strong className="text-lg">{isolate(t.station)}</strong>
                <StatusBadge value={t.status} tones={TICKET_TONES} />
              </div>
              <div className="text-base text-muted-foreground">{isolate(t.table_label)}</div>
              <ul className="stack text-lg">
                {(t.items ?? []).map((i, n) => (
                  <li key={n}>
                    {i.voided ? (
                      <span className="text-muted-foreground">
                        <s>
                          {i.qty} × {isolate(i.dish)}
                        </s>{isolate(" ")}
                        {tr("(voided)")}
                      </span>
                    ) : (
                      <>
                        {i.qty} × {isolate(i.dish)} {i.modifiers ? <strong>· {isolate(i.modifiers)}</strong> : null}{" "}
                        {i.note ? <span className="text-muted-foreground">({isolate(i.note)})</span> : null}
                      </>
                    )}
                  </li>
                ))}
              </ul>
              {next && (
                <Button className="kitchen-step" loading={move.isPending && move.variables?.id === t.id} onClick={() => move.mutate({ id: t.id, action: next.action })}>
                  {isolate(next.label)}
                </Button>
              )}
            </section>
          );
        })}
      </div>
    </PageStack>
  );
}
