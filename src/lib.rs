//! Restaurant (FERRUM-INDUSTRY-002): outlets, tables and menus; orders for dine-in, takeaway and
//! delivery; kitchen tickets by station; bills through ERP; and table reservations.
//!
//! * `orders`: an order's lines priced from its outlet's menu when taken; one open order to a table;
//!   new lines sent to the kitchen as a ticket for each station; what went to the kitchen is voided,
//!   not edited; billing makes the invoice and frees the table; a delivery goes out and arrives.
//! * `kitchen`: a station's ticket from queued through preparing and ready to served, its order's
//!   lines following.
//! * `reservations`: a table holds one party at a time, and no more than it seats.
//!
//! Adapted from ERPNext 12's Restaurant module (GPL-3.0; docs/PROVENANCE.md). Hooks are registered
//! per entity, so a workspace without the `restaurant` package never runs them.

mod kitchen;
mod orders;
mod reservations;

use ferrum_domain::reports::ReportSource;
use ferrum_kernel::{Doc, HookRegistry, Kernel, Result};
use serde_json::Value;
use std::sync::Arc;

include!(concat!(env!("OUT_DIR"), "/packages.rs"));
include!(concat!(env!("OUT_DIR"), "/ui.rs"));

/// This app's manifest (`app.yaml`).
pub const MANIFEST: &str = include_str!("../app.yaml");

/// This app's metadata packages, embedded at build time.
pub fn packages() -> std::result::Result<Vec<ferrum_meta::Package>, ferrum_meta::MetaError> {
    ferrum_meta::catalog::load_embedded(EMBEDDED)
}

/// Registers this app's document hooks.
pub fn register(hooks: &mut HookRegistry) {
    hooks
        .register("RestaurantOrder", Arc::new(orders::OrderHook))
        .register("KitchenTicket", Arc::new(kitchen::TicketHook))
        .register("TableReservation", Arc::new(reservations::ReservationHook));
}

/// This app's reports: none of its own yet; sales are ERP's.
pub fn reports() -> Vec<Arc<dyn ReportSource>> {
    vec![]
}

/// Upkeep: nothing runs by itself.
pub async fn maintenance(_k: &Kernel) -> Result<()> {
    Ok(())
}

/// A document from JSON.
pub(crate) fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap_or_default()
}
