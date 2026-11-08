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
  is voided and the dish ordered again, and a voided line is neither charged nor billed. It is
  struck off its ticket in the kitchen, and a ticket left with nothing to make is Voided. A line's
  state is the kitchen's to move on: a person only voids it. An order with something sent cannot be
  cancelled: bill it, or void those lines first.
- **Bills through ERP.** Billing makes a draft Sales Invoice of the order's lines, to its customer or
  the outlet's walk-in customer. The order closes and the table is free.
- **Takeaway and delivery.** Started from the floor. A delivery needs an address and a phone, goes
  out once the kitchen has made each dish, then arrives.
- **The floor starts an order.** A free table opens a new order for that table and its outlet; an
  empty restaurant says what to set up first. Each action says what it did.
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
| **0.2.7** | Its names in Arabic and Hindi. |
| **0.2.6** | The assistant knows its records. |
| **0.2.5** | Rebuilt with Ferrum-Frontend 5b22e7b: its own words in Arabic and Hindi. |
| **0.2.4** | Rebuilt with Ferrum-Frontend 6537d65: once-shown answers kept; English isolated right to left; its own words translatable by a workspace. |
| **0.2.3** | Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own interface words; pictures in attachment fields; a duplicate value said so. |
| **0.2.2** | Rebuilt with Ferrum-Frontend 63280d3: right to left, pages mirror; the records pages' words in Arabic and Hindi. |
| **0.2.1** | Rebuilt with Ferrum-Frontend a64416a: a box whose default is 1 shows ticked; action dialogs say every refusal and ask once. |
| **0.2.0** | What walking an evening found: a voided dish is struck off its kitchen ticket; a line's state only the kitchen moves on; a delivery goes out once made; the floor starts an order for its table, a takeaway or a delivery; dishes named as the menu writes them; actions say what they did. |
| **0.1.7** | Rebuilt with Ferrum-Frontend a76295e: times on a person's own clock when they chose one. |
| **0.1.6** | Rebuilt with Ferrum-Frontend 24c3951: times on the workspace's clock; drafts headed as drafts. |
| **0.1.5** | Rebuilt with Ferrum-Frontend bb60115: empty numbers read as the server reads them; settings open as their record. |
| **0.1.4** | Rebuilt with Ferrum-Frontend 00dea07: a workflow state's kept fields are read-only, its needed ones required. |
| **0.1.3** | Personal-data requests find reservations and orders by phone number, and erase them. |
| **0.1.2** | Rebuilt with Ferrum-Frontend daeadda: actions say what they did. |
| **0.1.1** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.0** | Tables and menus, orders for dine-in, takeaway and delivery, kitchen tickets by station, bills through ERP and reservations (FERRUM-INDUSTRY-002). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
