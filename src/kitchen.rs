//! Kitchen tickets: a station's part of an order goes from queued through preparing and ready to
//! served, and the order's lines on the ticket follow it (ready, then served). A line voided on the
//! order is struck off its ticket there (`orders::strike`).

use crate::obj;
use async_trait::async_trait;
use ferrum_domain::util::{elevated, id, rows, s, set};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result};
use serde_json::{Value, json};

pub struct TicketHook;

#[async_trait]
impl DocHook for TicketHook {
    fn name(&self) -> &'static str {
        "restaurant.ticket"
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        let (to, lines_to) = match name {
            "start" => ("Preparing", None),
            "ready" => ("Ready", Some("Ready")),
            "served" => ("Served", Some("Served")),
            _ => return Ok(None),
        };
        let ticket = id(doc, "id").ok_or_else(|| Error::internal("a ticket without an id"))?;
        let system = elevated(hc.ctx);
        hc.kernel.set_values_tx(&system, hc.tx, "KitchenTicket", ticket, obj(json!({"status": to}))).await?;
        if let (Some(state), Some(order)) = (lines_to, id(doc, "order")) {
            let on_ticket: Vec<String> = rows(doc, "items").iter().filter_map(|i| s(i, "line").map(str::to_string)).collect();
            let current = hc.kernel.get_tx(&system, hc.tx, "RestaurantOrder", order, true).await?;
            let mut lines = rows(&current, "lines");
            for l in lines.iter_mut().filter(|l| s(l, "id").is_some_and(|i| on_ticket.iter().any(|t| t == i)) && s(l, "state") != Some("Void")) {
                set(l, "state", json!(state));
            }
            hc.kernel.update_tx(&system, hc.tx, "RestaurantOrder", order, obj(json!({"lines": lines})), None).await?;
        }
        let what = match to {
            "Preparing" => "Preparing",
            "Ready" => "Ready",
            _ => "Served",
        };
        Ok(Some(json!({"status": to, "message": format!("{what} for {}", s(doc, "table_label").unwrap_or("the order"))})))
    }
}
