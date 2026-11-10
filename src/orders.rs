//! Orders: lines priced from the outlet's menu when they are taken, their choices (modifiers) on top
//! (ERPNext 12's Restaurant Order Entry priced from the active menu the same way); one open order to
//! a table; an order moved to another table, or merged into the order already there; new lines sent
//! to the kitchen as one ticket for each station; what went to the kitchen is voided rather than
//! edited, and a voided line is struck off its ticket; billing (`bills`) makes the invoices through
//! ERP and frees the table; a delivery goes out once the kitchen has made it, then arrives.

use crate::obj;
use async_trait::async_trait;
use ferrum_domain::flow::created;
use ferrum_domain::util::{d, elevated, id, money, rows, s, set};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct OrderHook;

/// A dish of the outlet's active menu: (item, price, station, available, its name on the menu).
async fn dish(tx: &mut Tx, tenant: Uuid, outlet: Uuid, name: &str) -> Result<Option<(Uuid, Decimal, String, bool, String)>> {
    Ok(sqlx::query_as(
        "select m.item, m.price, m.station, coalesce(m.available, false), m.dish_name from e_outlet o join e_menu__dishes m on m.tenant_id = o.tenant_id and m.parent_id = o.menu \
         where o.tenant_id = $1 and o.id = $2 and lower(m.dish_name) = lower($3) limit 1",
    )
    .bind(tenant)
    .bind(outlet)
    .bind(name.trim())
    .fetch_optional(&mut **tx)
    .await?)
}

/// A choice on the outlet's active menu for `dish` (one named for it before one for every dish):
/// (price, available, its name on the menu, the item it uses, how much of it for each dish).
pub(crate) async fn modifier(
    tx: &mut Tx,
    tenant: Uuid,
    outlet: Uuid,
    dish: &str,
    name: &str,
) -> Result<Option<(Decimal, bool, String, Option<Uuid>, Option<Decimal>)>> {
    Ok(sqlx::query_as(
        "select coalesce(m.price, 0), coalesce(m.available, false), m.modifier, m.item, m.qty from e_outlet o \
         join e_menu__modifiers m on m.tenant_id = o.tenant_id and m.parent_id = o.menu \
         where o.tenant_id = $1 and o.id = $2 and lower(m.modifier) = lower($3) and (coalesce(m.dish, '') = '' or lower(m.dish) = lower($4)) \
         order by coalesce(m.dish, '') = '' limit 1",
    )
    .bind(tenant)
    .bind(outlet)
    .bind(name.trim())
    .bind(dish.trim())
    .fetch_optional(&mut **tx)
    .await?)
}

/// A line's choices as it names them, in its order.
pub(crate) fn choices(l: &Doc) -> Vec<String> {
    s(l, "modifiers").unwrap_or_default().split(',').map(str::trim).filter(|m| !m.is_empty()).map(str::to_string).collect()
}

/// What a line's price is for: its dish and choices, in any case and order.
fn priced_key(dish: &str, choices: &[String]) -> String {
    let mut c: Vec<String> = choices.iter().map(|m| m.to_lowercase()).collect();
    c.sort();
    format!("{}|{}", dish.trim().to_lowercase(), c.join(","))
}

/// The lines that went to the kitchen: (id, dish).
fn sent(doc: &Doc) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = rows(doc, "lines")
        .iter()
        .filter(|l| s(l, "state").is_some_and(|st| st != "New"))
        .map(|l| (s(l, "id").unwrap_or_default().to_string(), s(l, "dish").unwrap_or_default().to_string()))
        .collect();
    out.sort();
    out
}

#[async_trait]
impl DocHook for OrderHook {
    fn name(&self) -> &'static str {
        "restaurant.order"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["lines", "total"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        let Some(outlet) = id(doc, "outlet") else { return Ok(()) };
        let mut total = Decimal::ZERO;
        let mut lines = rows(doc, "lines");
        for l in lines.iter_mut() {
            let qty = Decimal::from(l.get("qty").and_then(Value::as_i64).unwrap_or(1).max(1));
            // A line is priced when it is taken, its choices on top, and priced again while it is
            // new if its dish or choices change; the menu's later prices do not reach it.
            let name = s(l, "dish").unwrap_or_default().to_string();
            let wanted = choices(l);
            let new = s(l, "state").is_none_or(|st| st == "New");
            let changed = s(l, "priced_as").is_some_and(|p| p != priced_key(&name, &wanted));
            if l.get("price").is_none_or(Value::is_null) || (new && changed) {
                let (item, mut price, station, available, named) =
                    dish(hc.tx, hc.ctx.tenant, outlet, &name).await?.ok_or_else(|| Error::field("lines", format!("{name} is not on the menu")))?;
                if !available {
                    return Err(Error::field("lines", format!("{name} is not available today")));
                }
                let mut written = Vec::new();
                for m in &wanted {
                    let (extra, on, as_menu, _, _) = modifier(hc.tx, hc.ctx.tenant, outlet, &named, m)
                        .await?
                        .ok_or_else(|| Error::field("lines", format!("{m} is not a choice for {named}")))?;
                    if !on {
                        return Err(Error::field("lines", format!("{as_menu} is not available today")));
                    }
                    price += extra;
                    written.push(as_menu);
                }
                // As the menu writes them, so the ticket and the bill read the same.
                set(l, "priced_as", json!(priced_key(&named, &written)));
                set(l, "dish", json!(named));
                set(l, "modifiers", if written.is_empty() { Value::Null } else { json!(written.join(", ")) });
                set(l, "item", json!(item.to_string()));
                set(l, "price", money(price));
                set(l, "station", json!(station));
            }
            if l.get("state").is_none_or(Value::is_null) {
                set(l, "state", json!("New"));
            }
            let amount = d(l, "price") * qty;
            set(l, "amount", money(amount));
            if s(l, "state") != Some("Void") {
                total += amount;
            }
        }
        set(doc, "lines", Value::Array(lines.into_iter().map(Value::Object).collect()));
        set(doc, "total", money(total));
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        let tenant = hc.ctx.tenant;
        if !hc.ctx.is_system {
            // A line's state is the kitchen's to move on; a person only voids it.
            let was: BTreeMap<String, String> = old
                .map(|o| rows(o, "lines").iter().filter_map(|l| Some((s(l, "id")?.to_string(), s(l, "state").unwrap_or("New").to_string()))).collect())
                .unwrap_or_default();
            for l in rows(doc, "lines") {
                let (dish, now) = (s(&l, "dish").unwrap_or("A dish"), s(&l, "state").unwrap_or("New"));
                let before = s(&l, "id").and_then(|i| was.get(i)).map(String::as_str).unwrap_or("New");
                if now != before && now != "Void" {
                    return Err(Error::field("lines", format!("{dish} is {}: the kitchen moves it on, and Void takes it off", before.to_lowercase())));
                }
            }
        }
        if let Some(o) = old
            && !hc.ctx.is_system
        {
            if s(o, "status") != Some("Open") {
                return Err(Error::invalid(format!("this order is {}: it does not change", s(o, "status").unwrap_or("closed").to_lowercase())));
            }
            // What went to the kitchen stays as it went: its dish, choices, quantity and note are
            // read-only once sent (the server ignores a change to them), and the line is voided
            // rather than removed.
            let lines = rows(doc, "lines");
            if let Some((_, dish)) = sent(o).into_iter().find(|(line, _)| !lines.iter().any(|l| s(l, "id") == Some(line.as_str()))) {
                return Err(Error::field("lines", format!("{dish} went to the kitchen: void it rather than remove it")));
            }
        }
        let outlet = id(doc, "outlet").ok_or_else(|| Error::field("outlet", "is required"))?;
        match s(doc, "order_type").unwrap_or("Dine In") {
            "Dine In" => {
                let table = id(doc, "table").ok_or_else(|| Error::field("table", "a table's order needs its table"))?;
                let of: Option<Option<Uuid>> = sqlx::query_scalar("select outlet from e_dining_table where tenant_id = $1 and id = $2")
                    .bind(tenant)
                    .bind(table)
                    .fetch_optional(&mut **hc.tx)
                    .await?;
                if of.flatten() != Some(outlet) {
                    return Err(Error::field("table", "is another outlet's"));
                }
                if s(doc, "status") == Some("Open") {
                    let me = id(doc, "id").unwrap_or_else(Uuid::nil);
                    let other: Option<String> = sqlx::query_scalar(
                        "select number from e_restaurant_order where tenant_id = $1 and \"table\" = $2 and status = 'Open' and id <> $3 limit 1",
                    )
                    .bind(tenant)
                    .bind(table)
                    .bind(me)
                    .fetch_optional(&mut **hc.tx)
                    .await?;
                    if let Some(n) = other {
                        return Err(Error::field("table", format!("has an open order, {n}: add to it")));
                    }
                }
            }
            "Delivery" if s(doc, "delivery_address").is_none() || s(doc, "phone").is_none() => {
                return Err(Error::field("delivery_address", "a delivery needs an address and a phone"));
            }
            _ => {}
        }
        Ok(())
    }

    async fn after_save(&self, hc: &mut HookCtx<'_>, doc: &Doc, old: Option<&Doc>) -> Result<()> {
        strike(hc, doc, old).await?;
        if s(doc, "order_type") != Some("Dine In") {
            return Ok(());
        }
        let system = elevated(hc.ctx);
        let open = s(doc, "status") == Some("Open");
        if let Some(table) = id(doc, "table") {
            hc.kernel.set_values_tx(&system, hc.tx, "DiningTable", table, obj(json!({"occupied": open}))).await?;
        }
        if let Some(before) = old.and_then(|o| id(o, "table")).filter(|b| Some(*b) != id(doc, "table")) {
            hc.kernel.set_values_tx(&system, hc.tx, "DiningTable", before, obj(json!({"occupied": false}))).await?;
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        match name {
            "send_to_kitchen" => Ok(Some(send(hc, doc).await?)),
            "bill" => Ok(Some(crate::bills::bill(hc, doc, params).await?)),
            "settle" => Ok(Some(crate::bills::settle(hc, doc, params).await?)),
            "move_to" => Ok(Some(move_to(hc, doc, params).await?)),
            "cancel_order" => {
                if !sent(doc).is_empty() {
                    return Err(Error::invalid("some of it went to the kitchen: bill it, or void those lines first"));
                }
                finish(hc, doc, json!({"status": "Cancelled"})).await?;
                Ok(Some(json!({"status": "Cancelled", "message": format!("{} is cancelled", s(doc, "number").unwrap_or("The order"))})))
            }
            "dispatch" => {
                // A delivery goes out with its food: each dish made at its station first.
                if let Some(l) = rows(doc, "lines").iter().find(|l| !matches!(s(l, "state"), Some("Ready" | "Served" | "Void"))) {
                    return Err(Error::invalid(format!("{} is not ready yet: send it to the kitchen and wait for it", s(l, "dish").unwrap_or("A dish"))));
                }
                let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
                hc.kernel.set_values_tx(&elevated(hc.ctx), hc.tx, "RestaurantOrder", order, obj(json!({"delivery_status": "Out for Delivery"}))).await?;
                Ok(Some(json!({"delivery_status": "Out for Delivery", "message": format!("{} is out for delivery", s(doc, "number").unwrap_or("The order"))})))
            }
            "delivered" => {
                let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
                hc.kernel.set_values_tx(&elevated(hc.ctx), hc.tx, "RestaurantOrder", order, obj(json!({"delivery_status": "Delivered"}))).await?;
                Ok(Some(json!({"delivery_status": "Delivered", "message": format!("{} is delivered", s(doc, "number").unwrap_or("The order"))})))
            }
            _ => Ok(None),
        }
    }
}

/// Closes an order with `values`, and frees its table.
pub(crate) async fn finish(hc: &mut HookCtx<'_>, doc: &Doc, values: Value) -> Result<()> {
    let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
    let system = elevated(hc.ctx);
    hc.kernel.set_values_tx(&system, hc.tx, "RestaurantOrder", order, obj(values)).await?;
    if let (Some("Dine In"), Some(table)) = (s(doc, "order_type"), id(doc, "table")) {
        hc.kernel.set_values_tx(&system, hc.tx, "DiningTable", table, obj(json!({"occupied": false}))).await?;
    }
    Ok(())
}

/// `send_to_kitchen`: the new lines, as one ticket for each station; they are sent.
async fn send(hc: &mut HookCtx<'_>, doc: &Doc) -> Result<Value> {
    let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
    let mut lines = rows(doc, "lines");
    let mut by_station: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for l in lines.iter_mut().filter(|l| s(l, "state") == Some("New")) {
        by_station.entry(s(l, "station").unwrap_or("Kitchen").to_string()).or_default().push(json!({
            "line": s(l, "id"), "dish": s(l, "dish"), "qty": l.get("qty").cloned().unwrap_or(json!(1)), "modifiers": s(l, "modifiers"), "note": s(l, "note"),
        }));
        set(l, "state", json!("Sent"));
    }
    if by_station.is_empty() {
        return Err(Error::invalid("nothing new to send to the kitchen"));
    }
    let stations: Vec<String> = by_station.keys().cloned().collect();
    let system = elevated(hc.ctx);
    let label = match (s(doc, "order_type"), id(doc, "table")) {
        (Some("Dine In"), Some(table)) => table_label(hc.tx, hc.ctx.tenant, table).await?,
        (kind, _) => format!("{} {}", kind.unwrap_or("Order"), s(doc, "number").unwrap_or_default()),
    };
    let mut tickets = Vec::new();
    for (station, items) in by_station {
        let t = hc
            .kernel
            .insert_tx(
                &system,
                hc.tx,
                "KitchenTicket",
                obj(json!({"order": order.to_string(), "station": station, "table_label": label, "status": "Queued", "items": items})),
            )
            .await?;
        tickets.push(created("KitchenTicket", &t));
    }
    hc.kernel.update_tx(&system, hc.tx, "RestaurantOrder", order, obj(json!({"lines": lines})), None).await?;
    let to = match stations.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => stations.concat(),
    };
    Ok(json!({"tickets": tickets, "message": format!("Sent to {to}, for {label}")}))
}

/// The lines voided by this save that had gone to the kitchen are struck off their tickets still
/// to be served, and a ticket with nothing left to make is Voided: the kitchen stops making it.
async fn strike(hc: &mut HookCtx<'_>, doc: &Doc, old: Option<&Doc>) -> Result<()> {
    let Some(old) = old else { return Ok(()) };
    let sent_before: Vec<String> = sent(old).into_iter().map(|(i, _)| i).collect();
    let voided: Vec<String> = rows(doc, "lines")
        .iter()
        .filter(|l| s(l, "state") == Some("Void"))
        .filter_map(|l| s(l, "id").map(str::to_string))
        .filter(|i| sent_before.contains(i) && rows(old, "lines").iter().any(|o| s(o, "id") == Some(i.as_str()) && s(o, "state") != Some("Void")))
        .collect();
    let Some(order) = id(doc, "id").filter(|_| !voided.is_empty()) else { return Ok(()) };
    let system = elevated(hc.ctx);
    let open: Vec<Uuid> =
        sqlx::query_scalar("select id from e_kitchen_ticket where tenant_id = $1 and \"order\" = $2 and status in ('Queued', 'Preparing', 'Ready')")
            .bind(hc.ctx.tenant)
            .bind(order)
            .fetch_all(&mut **hc.tx)
            .await?;
    for ticket in open {
        let t = hc.kernel.get_tx(&system, hc.tx, "KitchenTicket", ticket, false).await?;
        let mut items = rows(&t, "items");
        if !items.iter().any(|i| s(i, "line").is_some_and(|l| voided.iter().any(|v| v == l))) {
            continue;
        }
        for i in items.iter_mut().filter(|i| s(i, "line").is_some_and(|l| voided.iter().any(|v| v == l))) {
            set(i, "voided", json!(true));
        }
        let left = items.iter().any(|i| i.get("voided") != Some(&json!(true)));
        let mut values = json!({"items": items});
        if !left {
            values["status"] = json!("Voided");
        }
        hc.kernel.update_tx(&system, hc.tx, "KitchenTicket", ticket, obj(values), None).await?;
    }
    Ok(())
}

/// How the kitchen names a table: a table named "Table 4" is not "Table Table 4".
async fn table_label(tx: &mut Tx, tenant: Uuid, table: Uuid) -> Result<String> {
    let name: Option<String> =
        sqlx::query_scalar("select table_name from e_dining_table where tenant_id = $1 and id = $2").bind(tenant).bind(table).fetch_optional(&mut **tx).await?;
    let name = name.unwrap_or_default();
    Ok(if name.to_lowercase().starts_with("table") { name } else { format!("Table {name}") })
}

/// `move_to`: the order to another table of its outlet. A free table takes the order; a table with
/// an open order takes its dishes and guests into that order, and this one closes as Merged,
/// pointing at it. Its kitchen tickets follow it, named for the new table.
async fn move_to(hc: &mut HookCtx<'_>, doc: &Doc, params: &Value) -> Result<Value> {
    let tenant = hc.ctx.tenant;
    let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
    let to: Uuid = params.get("table").and_then(Value::as_str).and_then(|t| t.parse().ok()).ok_or_else(|| Error::field("table", "is required"))?;
    if Some(to) == id(doc, "table") {
        return Err(Error::field("table", "is the order's table already"));
    }
    let of: Option<Option<Uuid>> =
        sqlx::query_scalar("select outlet from e_dining_table where tenant_id = $1 and id = $2").bind(tenant).bind(to).fetch_optional(&mut **hc.tx).await?;
    match of {
        None => return Err(Error::field("table", "is not a table")),
        Some(o) if o != id(doc, "outlet") => return Err(Error::field("table", "is another outlet's")),
        _ => {}
    }
    let label = table_label(hc.tx, tenant, to).await?;
    let there: Option<Uuid> =
        sqlx::query_scalar("select id from e_restaurant_order where tenant_id = $1 and \"table\" = $2 and status = 'Open' and id <> $3 limit 1")
            .bind(tenant)
            .bind(to)
            .bind(order)
            .fetch_optional(&mut **hc.tx)
            .await?;
    let system = elevated(hc.ctx);
    let tickets: Vec<Uuid> =
        sqlx::query_scalar("select id from e_kitchen_ticket where tenant_id = $1 and \"order\" = $2").bind(tenant).bind(order).fetch_all(&mut **hc.tx).await?;
    let Some(into) = there else {
        hc.kernel.update_tx(&system, hc.tx, "RestaurantOrder", order, obj(json!({"table": to.to_string()})), None).await?;
        for t in tickets {
            hc.kernel.set_values_tx(&system, hc.tx, "KitchenTicket", t, obj(json!({"table_label": label}))).await?;
        }
        return Ok(json!({"table": to.to_string(), "message": format!("Moved to {label}")}));
    };
    // Its lines go on as they are (prices, states); taken into another order they are new rows,
    // so its tickets are pointed at them.
    let target = hc.kernel.get_tx(&system, hc.tx, "RestaurantOrder", into, true).await?;
    let mut lines = rows(&target, "lines");
    let kept = lines.len();
    let moving = rows(doc, "lines");
    lines.extend(moving.iter().cloned().map(|mut l| {
        l.remove("id");
        l
    }));
    let guests = target.get("guests").and_then(Value::as_i64).unwrap_or(1) + doc.get("guests").and_then(Value::as_i64).unwrap_or(1);
    let merged = hc.kernel.update_tx(&system, hc.tx, "RestaurantOrder", into, obj(json!({"lines": lines, "guests": guests})), None).await?;
    let now = rows(&merged, "lines");
    let renamed: BTreeMap<String, String> =
        moving.iter().zip(now.iter().skip(kept)).filter_map(|(was, is)| Some((s(was, "id")?.to_string(), s(is, "id")?.to_string()))).collect();
    for t in tickets {
        let ticket = hc.kernel.get_tx(&system, hc.tx, "KitchenTicket", t, true).await?;
        let mut items = rows(&ticket, "items");
        for i in items.iter_mut() {
            if let Some(n) = s(i, "line").and_then(|l| renamed.get(l)) {
                set(i, "line", json!(n));
            }
        }
        hc.kernel.update_tx(&system, hc.tx, "KitchenTicket", t, obj(json!({"order": into.to_string(), "table_label": label, "items": items})), None).await?;
    }
    hc.kernel.update_tx(&system, hc.tx, "RestaurantOrder", order, obj(json!({"lines": [], "status": "Merged", "merged_into": into.to_string()})), None).await?;
    let number = s(&merged, "number").unwrap_or_default();
    Ok(json!({"merged_into": into.to_string(), "message": format!("Merged into {number} at {label}")}))
}
