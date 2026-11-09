//! Bills: an order billed through ERP as one invoice or one for each seat; the stock its dishes use,
//! taken from the outlet's store by their recipes; and payments at the table, by cash or card, with
//! a tip on top for the staff, or a bill put on a hotel guest's room where Hospitality is installed.

use crate::obj;
use crate::orders::{choices, finish, modifier};
use async_trait::async_trait;
use ferrum_domain::flow::{create_for_user, created};
use ferrum_domain::util::{d, elevated, id, money, rows, s, sys};
use ferrum_kernel::{Action, Doc, DocHook, Error, HookCtx, Result};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

/// "a, b and c".
fn and_join(parts: &[String]) -> String {
    match parts.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => parts.concat(),
    }
}

/// A money parameter of an action, when given.
fn amount(params: &Value, name: &str) -> Result<Option<Decimal>> {
    let text = match params.get(name) {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(t)) => t.trim().to_string(),
        Some(v) => v.to_string(),
    };
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<Decimal>().map(Some).map_err(|_| Error::field(name, "is an amount"))
}

/// `bill`: the order's invoices through ERP, its lines but the voided ones, to its customer or the
/// outlet's walk-in customer: one, or one for each seat (every line then has its seat). The stock
/// its dishes use is taken from the outlet's store; the order closes and its table is free.
pub(crate) async fn bill(hc: &mut HookCtx<'_>, doc: &Doc, params: &Value) -> Result<Value> {
    let tenant = hc.ctx.tenant;
    let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
    let outlet = hc.kernel.get_tx(&sys(hc.ctx), hc.tx, "Outlet", id(doc, "outlet").ok_or_else(|| Error::field("outlet", "is required"))?, false).await?;
    let lines: Vec<Doc> = rows(doc, "lines").into_iter().filter(|l| s(l, "state") != Some("Void")).collect();
    if lines.is_empty() {
        return Err(Error::invalid("nothing to bill: cancel the order instead"));
    }
    let by_seat = params.get("split").and_then(Value::as_str) == Some("By Seat");
    let mut groups: BTreeMap<Option<i64>, Vec<&Doc>> = BTreeMap::new();
    for l in &lines {
        let seat = match (by_seat, l.get("seat").and_then(Value::as_i64)) {
            (false, _) => None,
            (true, Some(n)) => Some(n),
            (true, None) => return Err(Error::invalid(format!("{} has no seat: give it one, or bill it as one", s(l, "dish").unwrap_or("A dish")))),
        };
        groups.entry(seat).or_default().push(l);
    }
    let company = id(&outlet, "company").ok_or_else(|| Error::invalid("the outlet has no company"))?;
    let customer = id(doc, "customer").or(id(&outlet, "walk_in_customer")).ok_or_else(|| Error::invalid("the outlet has no walk-in customer"))?;
    let currency: Option<Uuid> = sqlx::query_scalar("select default_currency from e_company where tenant_id = $1 and id = $2")
        .bind(tenant)
        .bind(company)
        .fetch_optional(&mut **hc.tx)
        .await?
        .flatten();
    let number = s(doc, "number").unwrap_or_default().to_string();
    let mut bills = Vec::new();
    let mut made = Vec::new();
    for (seat, ls) in &groups {
        let items: Vec<Value> = ls
            .iter()
            .map(|l| {
                let what = match s(l, "modifiers") {
                    Some(m) => format!("{} ({m})", s(l, "dish").unwrap_or_default()),
                    None => s(l, "dish").unwrap_or_default().to_string(),
                };
                json!({"item": s(l, "item"), "qty": l.get("qty").and_then(Value::as_i64).unwrap_or(1).to_string(), "price_list_rate": money(d(l, "price")),
                       "discount_percent": "0", "description": what})
            })
            .collect();
        let remarks = match seat {
            Some(n) => format!("Order {number}, seat {n}"),
            None => format!("Order {number}"),
        };
        let inv = create_for_user(
            hc,
            "SalesInvoice",
            obj(json!({"company": company.to_string(), "customer": customer.to_string(), "currency": currency.map(|c| c.to_string()), "items": items, "remarks": remarks})),
        )
        .await?;
        bills.push(obj(json!({"seat": seat, "sales_invoice": inv.get("id"), "amount": inv.get("grand_total"), "paid": "0", "tips": "0", "status": "Unpaid"})));
        made.push(inv);
    }
    let used = consume(hc, &number, &outlet, &lines).await?;
    let system = elevated(hc.ctx);
    hc.kernel.set_child_rows_tx(&system, hc.tx, "RestaurantOrder", "bills", order, bills).await?;
    let one = if made.len() == 1 { made[0].get("id").cloned().unwrap_or(Value::Null) } else { Value::Null };
    let stock = used.as_ref().and_then(|u| u.get("id").cloned()).unwrap_or(Value::Null);
    finish(hc, doc, json!({"status": "Billed", "sales_invoice": one, "stock_entry": stock})).await?;
    let freed = if s(doc, "order_type") == Some("Dine In") { "; the table is free" } else { "" };
    let total = |inv: &Doc| inv.get("grand_total").and_then(Value::as_str).unwrap_or_default().to_string();
    if let [inv] = made.as_slice() {
        return Ok(
            json!({"status": "Billed", "sales_invoice": created("SalesInvoice", inv), "message": format!("Billed {}: a draft invoice in ERP{freed}", total(inv))}),
        );
    }
    let each: Vec<String> = groups.keys().zip(&made).map(|(seat, inv)| format!("seat {} {}", seat.unwrap_or_default(), total(inv))).collect();
    Ok(json!({"status": "Billed", "sales_invoices": made.iter().map(|i| created("SalesInvoice", i)).collect::<Vec<_>>(),
              "message": format!("Billed by seat, {} draft invoices in ERP: {}{freed}", made.len(), and_join(&each))}))
}

/// What the dishes billed take from the outlet's store: each dish's recipe (its item's bill of
/// materials in ERP, the default one else the newest active, scrap included, as ERP's manufacturing
/// takes it), or the dish's item itself when it is a stock item without one, with what its choices
/// add or leave out; issued from the store as one Material Issue, submitted, at the company's cost
/// of goods sold. Nothing when the outlet keeps stock out of it or nothing billed uses stock.
async fn consume(hc: &mut HookCtx<'_>, number: &str, outlet: &Doc, lines: &[Doc]) -> Result<Option<Doc>> {
    let Some(store) = id(outlet, "store") else { return Ok(None) };
    let tenant = hc.ctx.tenant;
    let company = id(outlet, "company").ok_or_else(|| Error::invalid("the outlet has no company"))?;
    let outlet_id = id(outlet, "id").ok_or_else(|| Error::internal("an outlet without an id"))?;
    let mut need: BTreeMap<Uuid, Decimal> = BTreeMap::new();
    for l in lines {
        let Some(item) = id(l, "item") else { continue };
        let qty = Decimal::from(l.get("qty").and_then(Value::as_i64).unwrap_or(1));
        let recipe: Option<(Uuid, Decimal)> = sqlx::query_as(
            "select id, quantity from e_bom where tenant_id = $1 and company = $2 and item = $3 and docstatus = 1 and is_active \
             order by is_default desc, created_at desc limit 1",
        )
        .bind(tenant)
        .bind(company)
        .bind(item)
        .fetch_optional(&mut **hc.tx)
        .await?;
        match recipe {
            Some((bom, makes)) => {
                let parts: Vec<(Uuid, Decimal, Option<Decimal>)> =
                    sqlx::query_as("select item, qty, scrap_percent from e_bom__items where tenant_id = $1 and parent_id = $2 order by idx")
                        .bind(tenant)
                        .bind(bom)
                        .fetch_all(&mut **hc.tx)
                        .await?;
                let makes = if makes.is_zero() { Decimal::ONE } else { makes };
                for (part, per, scrap) in parts {
                    let with_scrap = Decimal::ONE + scrap.unwrap_or_default() / Decimal::ONE_HUNDRED;
                    *need.entry(part).or_default() += per / makes * qty * with_scrap;
                }
            }
            None => {
                let stocked: Option<bool> = sqlx::query_scalar("select is_stock_item from e_item where tenant_id = $1 and id = $2")
                    .bind(tenant)
                    .bind(item)
                    .fetch_optional(&mut **hc.tx)
                    .await?
                    .flatten();
                if stocked == Some(true) {
                    *need.entry(item).or_default() += qty;
                }
            }
        }
        let dish = s(l, "dish").unwrap_or_default().to_string();
        for m in choices(l) {
            if let Some((_, _, _, Some(part), Some(per))) = modifier(hc.tx, tenant, outlet_id, &dish, &m).await? {
                *need.entry(part).or_default() += per * qty;
            }
        }
    }
    need.retain(|_, q| *q > Decimal::ZERO);
    if need.is_empty() {
        return Ok(None);
    }
    let system = elevated(hc.ctx);
    let cogs = id(&hc.kernel.get_tx(&system, hc.tx, "Company", company, false).await?, "cogs_account");
    let items: Vec<Value> = need
        .iter()
        .map(|(item, qty)| {
            json!({"item": item.to_string(), "qty": qty.round_dp(9).normalize().to_string(), "source_warehouse": store.to_string(),
                   "expense_account": cogs.map(|c| c.to_string())})
        })
        .collect();
    let entry = hc
        .kernel
        .insert_tx(
            &system,
            hc.tx,
            "StockEntry",
            obj(
                json!({"company": company.to_string(), "purpose": "Material Issue", "items": items, "remarks": format!("Order {number}: what its dishes use")}),
            ),
        )
        .await?;
    let eid = id(&entry, "id").ok_or_else(|| Error::internal("a stock entry without an id"))?;
    hc.kernel.submit_tx(&system, hc.tx, "StockEntry", eid, None).await.map_err(|e| match e {
        Error::Invalid(m) => Error::invalid(format!("the store does not hold what its dishes use: {m}")),
        other => other,
    })?;
    Ok(Some(entry))
}

/// Whether every bill of an order is done with: paid, or on a hotel room.
fn all_settled(bills: &[Doc]) -> bool {
    bills.iter().all(|b| matches!(s(b, "status"), Some("Paid" | "On Room")))
}

/// Paid by Room: the bill goes on a hotel guest's room as one of Hospitality's room charges, which
/// checks the guest in the room and posts it to their folio; putting it there is an Outlet
/// Cashier's right. The hotel invoices it with the room (as the outlet's room charge item), so the
/// bill's own draft invoice is dropped. A bill goes on a room whole, without a tip, and not after
/// part of it was paid here.
async fn on_room(hc: &mut HookCtx<'_>, doc: &Doc, bill: &mut Doc, n: i64, inv_id: Uuid, params: &Value) -> Result<String> {
    if hc.ctx.schema.entity("RoomCharge").is_none() {
        return Err(Error::invalid("putting a bill on a room needs Hospitality"));
    }
    if amount(params, "tip")?.is_some_and(|t| t > Decimal::ZERO) {
        return Err(Error::field("tip", "is not put on a room: take it by cash or card"));
    }
    if amount(params, "amount")?.is_some() {
        return Err(Error::field("amount", "a bill goes on a room whole"));
    }
    let text = |k: &str| params.get(k).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let number = text("room").ok_or_else(|| Error::field("room", "which room?"))?;
    let guest = text("guest").ok_or_else(|| Error::field("guest", "the guest's name, as they give it"))?;
    let system = elevated(hc.ctx);
    let inv = hc.kernel.get_tx(&system, hc.tx, "SalesInvoice", inv_id, true).await?;
    if inv.get("docstatus").and_then(Value::as_i64) != Some(0) || d(bill, "paid") > Decimal::ZERO {
        return Err(Error::invalid(format!("bill {n} was paid here in part: only a bill not paid at all goes on a room")));
    }
    let outlet = hc.kernel.get_tx(&sys(hc.ctx), hc.tx, "Outlet", id(doc, "outlet").ok_or_else(|| Error::field("outlet", "is required"))?, false).await?;
    let hotel = s(&outlet, "hotel_code").ok_or_else(|| Error::invalid("the outlet names no hotel: set its Hotel Code to put bills on rooms"))?.to_string();
    let item = id(&outlet, "room_charge_item").ok_or_else(|| Error::invalid("the outlet has no Room Charge Item: what the hotel invoices its bills as"))?;
    let found: Option<(Uuid, Uuid)> = sqlx::query_as(
        "select p.id, r.id from e_hotel_property p join e_room r on r.tenant_id = p.tenant_id and r.property = p.id \
         where p.tenant_id = $1 and p.code = $2 and r.room_number = $3",
    )
    .bind(hc.ctx.tenant)
    .bind(&hotel)
    .bind(&number)
    .fetch_optional(&mut **hc.tx)
    .await?;
    let (property, room) = found.ok_or_else(|| Error::field("room", format!("is not a room of hotel {hotel}")))?;
    let total = d(&inv, "grand_total");
    let order = s(doc, "number").unwrap_or_default().to_string();
    let charge = hc
        .kernel
        .insert_tx(
            hc.ctx,
            hc.tx,
            "RoomCharge",
            obj(json!({"property": property.to_string(), "room": room.to_string(), "guest_name": guest, "outlet": s(&outlet, "outlet_name"),
                       "description": format!("Order {order}, bill {n}"), "amount": money(total), "item": item.to_string(), "reference": order})),
        )
        .await?;
    // Unlinked from the order below, then dropped.
    bill.insert("sales_invoice".into(), Value::Null);
    bill.insert("room_charge".into(), charge.get("id").cloned().unwrap_or(Value::Null));
    bill.insert("amount".into(), money(total));
    bill.insert("status".into(), json!("On Room"));
    Ok(format!("Put on room {number}: bill {n}, {total:.2}"))
}

/// `settle`: a payment at the table against one of the order's bills, through ERP. Its invoice is
/// submitted if it is still a draft, and a payment entry made for what this payment covers (all the
/// bill still owes, or part of it), by cash or card into the outlet's account, with a tip on top
/// credited to the outlet's tips account for the staff; or the bill goes on a hotel room
/// (`on_room`). Every bill done with, the order is Settled.
pub(crate) async fn settle(hc: &mut HookCtx<'_>, doc: &Doc, params: &Value) -> Result<Value> {
    let order = id(doc, "id").ok_or_else(|| Error::internal("an order without an id"))?;
    // Its own fields only: the rows are written back as they are kept.
    const KEPT: [&str; 8] = ["id", "seat", "sales_invoice", "amount", "paid", "tips", "status", "room_charge"];
    let mut bills: Vec<Doc> = rows(doc, "bills")
        .into_iter()
        .map(|mut b| {
            b.retain(|k, _| KEPT.contains(&k.as_str()));
            b
        })
        .collect();
    // Billed before bills were kept: its one invoice.
    if bills.is_empty()
        && let Some(inv) = id(doc, "sales_invoice")
    {
        bills.push(obj(json!({"sales_invoice": inv.to_string(), "paid": "0", "tips": "0", "status": "Unpaid"})));
    }
    let count = bills.len();
    let n = params.get("bill").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|t| t.trim().parse().ok()))).unwrap_or(1);
    let Some(at) = usize::try_from(n).ok().filter(|n| (1..=count).contains(n)).map(|n| n - 1) else {
        return Err(Error::field("bill", format!("is from 1 to {count}, as numbered on the order")));
    };
    if s(&bills[at], "status") == Some("On Room") {
        return Err(Error::invalid(format!("bill {n} is on a hotel room")));
    }
    let inv_id = id(&bills[at], "sales_invoice").ok_or_else(|| Error::internal("a bill without its invoice"))?;
    if params.get("paid_by").and_then(Value::as_str) == Some("Room") {
        let said = on_room(hc, doc, &mut bills[at], n, inv_id, params).await?;
        let settled = all_settled(&bills);
        let system = elevated(hc.ctx);
        let one = (id(doc, "sales_invoice") == Some(inv_id)).then_some(Value::Null);
        hc.kernel.set_child_rows_tx(&system, hc.tx, "RestaurantOrder", "bills", order, bills).await?;
        let mut values = json!({});
        if settled {
            values["status"] = json!("Settled");
        }
        if let Some(none) = one {
            values["sales_invoice"] = none;
        }
        hc.kernel.set_values_tx(&system, hc.tx, "RestaurantOrder", order, obj(values)).await?;
        hc.kernel.delete_tx(&system, hc.tx, "SalesInvoice", inv_id, None).await?;
        let done = if settled { "; the order is settled" } else { "" };
        return Ok(json!({"status": if settled { "Settled" } else { "Billed" }, "message": format!("{said}{done}")}));
    }
    hc.ctx.require("PaymentEntry", Action::Create)?;
    let system = elevated(hc.ctx);
    let inv = hc.kernel.get_tx(&system, hc.tx, "SalesInvoice", inv_id, true).await?;
    match inv.get("docstatus").and_then(Value::as_i64) {
        Some(0) => {
            hc.kernel.submit_tx(&system, hc.tx, "SalesInvoice", inv_id, None).await?;
        }
        Some(2) => return Err(Error::invalid(format!("bill {n}'s invoice is cancelled in ERP"))),
        _ => {}
    }
    let inv = hc.kernel.get_tx(&system, hc.tx, "SalesInvoice", inv_id, false).await?;
    let owes = d(&inv, "outstanding_amount");
    if owes <= Decimal::ZERO {
        return Err(Error::invalid(format!("bill {n} is paid")));
    }
    let paying = amount(params, "amount")?.unwrap_or(owes);
    if paying <= Decimal::ZERO {
        return Err(Error::field("amount", "is above zero"));
    }
    if paying > owes {
        return Err(Error::field("amount", format!("bill {n} owes {owes:.2}")));
    }
    let tip = amount(params, "tip")?.unwrap_or_default();
    if tip < Decimal::ZERO {
        return Err(Error::field("tip", "is not below zero"));
    }
    let outlet = hc.kernel.get_tx(&sys(hc.ctx), hc.tx, "Outlet", id(doc, "outlet").ok_or_else(|| Error::field("outlet", "is required"))?, false).await?;
    let by = match params.get("paid_by").and_then(Value::as_str) {
        Some("Card") => "card",
        _ => "cash",
    };
    let into = id(&outlet, if by == "card" { "card_account" } else { "cash_account" })
        .ok_or_else(|| Error::invalid(format!("the outlet has no {by} account: set one to take payments by {by}")))?;
    let number = s(doc, "number").unwrap_or_default().to_string();
    let mut deductions = Vec::new();
    if tip > Decimal::ZERO {
        let tips = id(&outlet, "tips_account").ok_or_else(|| Error::invalid("the outlet has no tips account: set one to take tips"))?;
        deductions.push(json!({"account": tips.to_string(), "amount": money(-tip), "remarks": format!("Tip, order {number}")}));
    }
    let pe = hc
        .kernel
        .insert_tx(
            &system,
            hc.tx,
            "PaymentEntry",
            obj(json!({"company": s(&inv, "company"), "payment_type": "Receive", "customer": s(&inv, "customer"), "bank_account": into.to_string(),
                       "currency": s(&inv, "currency"), "paid_amount": money(paying + tip),
                       "references": [{"invoice": inv_id.to_string(), "allocated_amount": money(paying)}], "deductions": deductions,
                       "remarks": format!("Order {number}, bill {n}, by {by}")})),
        )
        .await?;
    let pid = id(&pe, "id").ok_or_else(|| Error::internal("a payment without an id"))?;
    hc.kernel.submit_tx(&system, hc.tx, "PaymentEntry", pid, None).await?;
    let left = owes - paying;
    let b = &mut bills[at];
    b.insert("paid".into(), money(d(b, "paid") + paying));
    b.insert("tips".into(), money(d(b, "tips") + tip));
    b.insert("amount".into(), inv.get("grand_total").cloned().unwrap_or(Value::Null));
    b.insert("status".into(), json!(if left.is_zero() { "Paid" } else { "Partly Paid" }));
    let settled = all_settled(&bills);
    hc.kernel.set_child_rows_tx(&system, hc.tx, "RestaurantOrder", "bills", order, bills).await?;
    if settled {
        hc.kernel.set_values_tx(&system, hc.tx, "RestaurantOrder", order, obj(json!({"status": "Settled"}))).await?;
    }
    let tipped = if tip > Decimal::ZERO { format!(" and a tip of {tip:.2}") } else { String::new() };
    let stands = if left.is_zero() { format!("bill {n} is paid") } else { format!("bill {n} owes {left:.2}") };
    let done = if settled { "; the order is settled" } else { "" };
    Ok(json!({"status": if settled { "Settled" } else { "Billed" }, "payment_entry": created("PaymentEntry", &pe),
              "message": format!("Took {paying:.2} by {by}{tipped}: {stands}{done}")}))
}

pub struct OutletHook;

#[async_trait]
impl DocHook for OutletHook {
    fn name(&self) -> &'static str {
        "restaurant.outlet"
    }

    /// Its store and accounts are its company's.
    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        let company = id(doc, "company");
        for (field, table) in [("store", "e_warehouse"), ("cash_account", "e_account"), ("card_account", "e_account"), ("tips_account", "e_account")] {
            let Some(of) = id(doc, field) else { continue };
            let sql = format!("select company from {table} where tenant_id = $1 and id = $2");
            let theirs: Option<Option<Uuid>> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql)).bind(hc.ctx.tenant).bind(of).fetch_optional(&mut **hc.tx).await?;
            if theirs.flatten() != company {
                return Err(Error::field(field, "is another company's"));
            }
        }
        Ok(())
    }
}
