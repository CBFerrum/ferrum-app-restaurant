# Restaurant

A restaurant for [Ferrum](https://github.com/CBFerrum/Ferrum):
- outlets, tables and menus;
- orders for dine-in, takeaway and delivery, and kitchen tickets by station;
- bills through ERP, and table reservations.

| | |
|---|---|
| 🍽️ App | `restaurant`, native, in the server |
| 🧩 Package | `restaurant` (needs ERP's selling and accounting) |
| 🖥️ Interface | its own, at `/restaurant/`: the floor and the kitchen, and the records |
| 🔐 Roles | Restaurant Manager, Server, Kitchen |
| 📜 Licence | AGPL-3.0-only |

## Highlights

- **Priced from the menu.** An order's lines are priced from the outlet's active menu when they are
  taken, and keep that price. A dish not on the menu, or not available today, is refused.
- **One open order to a table.** The table shows as taken until the order is billed or cancelled.
- **The kitchen by station.** Sending an order makes one ticket for each station (grill, kitchen,
  bar, pastry) with its new lines. A ticket goes from queued through preparing and ready to served,
  and the order's lines follow it.
- **What went to the kitchen is voided, not edited.** A sent line cannot be changed or removed. It
  is voided and the dish ordered again, and a voided line is neither charged nor billed. An order
  with something sent cannot be cancelled: bill it, or void those lines first.
- **Bills through ERP.** Billing makes a draft Sales Invoice of the order's lines, to its customer or
  the outlet's walk-in customer. The order closes and the table is free.
- **Takeaway and delivery.** A delivery needs an address and a phone, and goes out, then arrives.
- **Reservations.** A table holds one party at a time, and no more than it seats. A cancelled booking
  frees the time.

## Develop

```bash
cargo test                       # needs FERRUM_RESTAURANT_TEST_DATABASE_URL's database
cd ui && npm run build && npm run check
```

`tests/restaurant_it.rs` walks an evening:
- a table's order through the kitchen to the bill;
- a voided line;
- a takeaway, and a delivery;
- reservations;
- who may do what.

## Not yet

- Modifiers and combos, splitting a bill, tips, and payment at the table (ERP settles the invoice).
- Stock used by recipes, and delivery platforms.
- Kitchen display screens beyond the page, and printers.

## Changelog

| Version | Highlights |
|---|---|
| **0.1.5** | Rebuilt with Ferrum-Frontend bb60115: empty numbers read as the server reads them; settings open as their record. |
| **0.1.4** | Rebuilt with Ferrum-Frontend 00dea07: a workflow state's kept fields are read-only, its needed ones required. |
| **0.1.3** | Personal-data requests find reservations and orders by phone number, and erase them. |
| **0.1.2** | Rebuilt with Ferrum-Frontend daeadda: actions say what they did. |
| **0.1.1** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.0** | Tables and menus, orders for dine-in, takeaway and delivery, kitchen tickets by station, bills through ERP and reservations (FERRUM-INDUSTRY-002). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
