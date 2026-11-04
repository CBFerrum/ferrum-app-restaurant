//! Orders: lines priced from the outlet's menu when they are taken (ERPNext 12's Restaurant Order
//! Entry priced from the active menu the same way); one open order to a table; new lines sent to the
//! kitchen as one ticket for each station; what went to the kitchen is voided rather than edited;
//! billing makes the invoice through ERP and frees the table; a delivery goes out, then arrives.

use crate::obj;
use async_trait::async_trait;
use ferrum_domain::flow::{create_for_user, created};
use ferrum_domain::util::{d, elevated, id, money, rows, s, set, sys};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct OrderHook;

/// A dish of the outlet's active menu: (item, price, station, available).
async fn dish(tx: &mut Tx, tenant: Uuid, outlet: Uuid, name: &str) -> Result<Option<(Uuid, Decimal, String, bool)>> {
    Ok(sqlx::query_as(
        "select m.item, m.price, m.station, coalesce(m.available, false) from e_outlet o join e_menu__dishes m on m.tenant_id = o.tenant_id and m.parent_id = o.menu \
         where o.tenant_id = $1 and o.id = $2 and lower(m.dish_name) = lower($3) limit 1",
    )
    .bind(tenant)
    .bind(outlet)
    .bind(name.trim())
    .fetch_optional(&mut **tx)
    .await?)
}

/// The lines that went to the kitchen: (id, dish, qty).
fn sent(doc: &Doc) -> Vec<(String, String, i64)> {
    let mut out: Vec<(String, String, i64)> = rows(doc, "lines")
        .iter()
        .filter(|l| s(l, "state").is_some_and(|st| st != "New"))
        .map(|l| (s(l, "id").unwrap_or_default().to_string(), s(l, "dish").unwrap_or_default().to_string(), l.get("qty").and_then(Value::as_i64).unwrap_or(0)))
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
            // A line is priced once, when it is taken.
            if l.get("price").is_none_or(Value::is_null) {
                let name = s(l, "dish").unwrap_or_default().to_string();
                let (item, price, station, available) =
                    dish(hc.tx, hc.ctx.tenant, outlet, &name).await?.ok_or_else(|| Error::field("lines", format!("{name} is not on the menu")))?;
                if !available {
                    return Err(Error::field("lines", format!("{name} is not available today")));
                }
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
        if let Some(o) = old
            && !hc.ctx.is_system
        {
            if s(o, "status") != Some("Open") {
                return Err(Error::invalid(format!("this order is {}: it does not change", s(o, "status").unwrap_or("closed").to_lowercase())));
            }
            // What went to the kitchen stays as it went: void a line rather than change it.
            let before: BTreeMap<String, (String, i64)> = sent(o).into_iter().map(|(i, d, q)| (i, (d, q))).collect();
            let lines = rows(doc, "lines");
            for (line, (dish, qty)) in &before {
                match lines.iter().find(|l| s(l, "id") == Some(line.as_str())) {
                    None => return Err(Error::field("lines", format!("{dish} went to the kitchen: void it rather than remove it"))),
                    Some(l) if s(l, "dish") != Some(dish.as_str()) || l.get("qty").and_then(Value::as_i64) != Some(*qty) => {
                        return Err(Error::field("lines", format!("{dish} went to the kitchen: void it and order again")));
                    }
                    _ => {}
                }
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

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        match name {
            "send_to_kitchen" => Ok(Some(send(hc, doc).await?)),
            "bill" => Ok(Some(bill(hc, doc).await?)),
            "cancel_order" => {
                if !sent(doc).is_empty() {
                    return Err(Error::invalid("some of it went to the kitchen: bill it, or void those lines first"));
                }
                finish(hc, doc, json!({"status": "Cancelled"})).await?;
                Ok(Some(json!({"status": "Cancelled"})))
            }
            "dispatch" => {
                let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
                hc.kernel.set_values_tx(&elevated(hc.ctx), hc.tx, "RestaurantOrder", order, obj(json!({"delivery_status": "Out for Delivery"}))).await?;
                Ok(Some(json!({"delivery_status": "Out for Delivery"})))
            }
            "delivered" => {
                let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
                hc.kernel.set_values_tx(&elevated(hc.ctx), hc.tx, "RestaurantOrder", order, obj(json!({"delivery_status": "Delivered"}))).await?;
                Ok(Some(json!({"delivery_status": "Delivered"})))
            }
            _ => Ok(None),
        }
    }
}

/// Closes an order with `values`, and frees its table.
async fn finish(hc: &mut HookCtx<'_>, doc: &Doc, values: Value) -> Result<()> {
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
            "line": s(l, "id"), "dish": s(l, "dish"), "qty": l.get("qty").cloned().unwrap_or(json!(1)), "note": s(l, "note"),
        }));
        set(l, "state", json!("Sent"));
    }
    if by_station.is_empty() {
        return Err(Error::invalid("nothing new to send to the kitchen"));
    }
    let system = elevated(hc.ctx);
    let label = match (s(doc, "order_type"), id(doc, "table")) {
        (Some("Dine In"), Some(table)) => {
            let name: Option<String> = sqlx::query_scalar("select table_name from e_dining_table where tenant_id = $1 and id = $2")
                .bind(hc.ctx.tenant)
                .bind(table)
                .fetch_optional(&mut **hc.tx)
                .await?;
            format!("Table {}", name.unwrap_or_default())
        }
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
    Ok(json!({"tickets": tickets}))
}

/// `bill`: the order's invoice through ERP, its lines but the voided ones, to its customer or the
/// outlet's walk-in customer; the order closes and its table is free.
async fn bill(hc: &mut HookCtx<'_>, doc: &Doc) -> Result<Value> {
    let tenant = hc.ctx.tenant;
    let outlet = hc.kernel.get_tx(&sys(hc.ctx), hc.tx, "Outlet", id(doc, "outlet").ok_or_else(|| Error::field("outlet", "is required"))?, false).await?;
    let lines: Vec<Doc> = rows(doc, "lines").into_iter().filter(|l| s(l, "state") != Some("Void")).collect();
    if lines.is_empty() {
        return Err(Error::invalid("nothing to bill: cancel the order instead"));
    }
    let company = id(&outlet, "company").ok_or_else(|| Error::invalid("the outlet has no company"))?;
    let customer = id(doc, "customer").or(id(&outlet, "walk_in_customer")).ok_or_else(|| Error::invalid("the outlet has no walk-in customer"))?;
    let currency: Option<Uuid> = sqlx::query_scalar("select default_currency from e_company where tenant_id = $1 and id = $2")
        .bind(tenant)
        .bind(company)
        .fetch_optional(&mut **hc.tx)
        .await?
        .flatten();
    let items: Vec<Value> = lines
        .iter()
        .map(|l| {
            json!({"item": s(l, "item"), "qty": l.get("qty").and_then(Value::as_i64).unwrap_or(1).to_string(), "price_list_rate": money(d(l, "price")), "discount_percent": "0",
                   "description": s(l, "dish")})
        })
        .collect();
    let inv = create_for_user(
        hc,
        "SalesInvoice",
        obj(json!({"company": company.to_string(), "customer": customer.to_string(), "currency": currency.map(|c| c.to_string()), "items": items,
                   "remarks": format!("Order {}", s(doc, "number").unwrap_or_default())})),
    )
    .await?;
    finish(hc, doc, json!({"status": "Billed", "sales_invoice": inv.get("id").cloned().unwrap_or(Value::Null)})).await?;
    Ok(json!({"status": "Billed", "sales_invoice": created("SalesInvoice", &inv)}))
}
