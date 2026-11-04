//! Reservations: a table holds one party at a time, no more than it seats (ERPNext 12's Restaurant
//! Reservation kept a time and a party; the table and the checks are this app's).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ferrum_domain::util::{id, s};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result};
use serde_json::Value;
use uuid::Uuid;

fn at(doc: &Doc, k: &str) -> Result<DateTime<Utc>> {
    s(doc, k).and_then(|v| DateTime::parse_from_rfc3339(v).ok()).map(|t| t.with_timezone(&Utc)).ok_or_else(|| Error::field(k, "is a date and time"))
}

pub struct ReservationHook;

#[async_trait]
impl DocHook for ReservationHook {
    fn name(&self) -> &'static str {
        "restaurant.reservation"
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        let tenant = hc.ctx.tenant;
        let (from, to) = (at(doc, "starts_at")?, at(doc, "ends_at")?);
        if to <= from {
            return Err(Error::field("ends_at", "is after it starts"));
        }
        let table = id(doc, "table").ok_or_else(|| Error::field("table", "is required"))?;
        let row: Option<(Option<Uuid>, i64)> = sqlx::query_as("select outlet, seats from e_dining_table where tenant_id = $1 and id = $2")
            .bind(tenant)
            .bind(table)
            .fetch_optional(&mut **hc.tx)
            .await?;
        let (outlet, seats) = row.ok_or_else(|| Error::field("table", "is not a table here"))?;
        if outlet != id(doc, "outlet") {
            return Err(Error::field("table", "is another outlet's"));
        }
        let party = doc.get("party").and_then(Value::as_i64).unwrap_or(1);
        if party > seats {
            return Err(Error::field("party", format!("the table seats {seats}")));
        }
        if matches!(s(doc, "status"), Some("Cancelled" | "No Show")) {
            return Ok(());
        }
        let me = id(doc, "id").unwrap_or_else(Uuid::nil);
        let other: Option<String> = sqlx::query_scalar(
            "select guest_name from e_table_reservation where tenant_id = $1 and \"table\" = $2 and id <> $3 and status in ('Booked', 'Seated') \
             and starts_at < $5 and ends_at > $4 limit 1",
        )
        .bind(tenant)
        .bind(table)
        .bind(me)
        .bind(from)
        .bind(to)
        .fetch_optional(&mut **hc.tx)
        .await?;
        if let Some(guest) = other {
            return Err(Error::field("table", format!("is held for {guest} then")));
        }
        Ok(())
    }
}
