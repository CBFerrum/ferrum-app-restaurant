<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/banner-dark.svg">
  <img alt="Ferrum Restaurant: from the table to the kitchen to the bill" src="docs/assets/banner-light.svg" width="100%">
</picture>

<br>

[![Version](https://img.shields.io/badge/version-0.3.3-b45309?style=flat-square)](CHANGELOG.md)
[![Licence: AGPL-3.0](https://img.shields.io/badge/licence-AGPL--3.0-blue?style=flat-square)](LICENSE)
[![Ferrum platform](https://img.shields.io/badge/runs%20on-Ferrum-2f5bea?style=flat-square)](https://github.com/CBFerrum/Ferrum)
![Rust](https://img.shields.io/badge/Rust-server-b7410e?style=flat-square&logo=rust)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-data-336791?style=flat-square&logo=postgresql&logoColor=white)
![React](https://img.shields.io/badge/React-TypeScript-087ea4?style=flat-square&logo=react&logoColor=white)
![Tests](https://img.shields.io/badge/tests-1%20Rust%20%C2%B7%206%20UI-16a34a?style=flat-square)

### From the table to the kitchen to the bill, with the stock and the books kept for you.

Ferrum Restaurant is for restaurants, cafés and bars that take orders for the table, for takeaway and for delivery. A menu prices every order, the kitchen gets a ticket for each station, and the bill is an ERP invoice paid at the table. It is an alternative to Toast, Square for Restaurants, Lightspeed Restaurant, TouchBistro or Odoo POS Restaurant for a place that wants the kitchen and the books on one self-hosted system.

[💡 Why](#-why-restaurant) · [📸 Screenshots](#-screenshots) · [✨ Features](#-features) · [📊 Compare](#-how-it-compares) · [📍 Roadmap](#-roadmap) · [🚀 Install](#-install) · [🔧 Develop](#-develop)

</div>

## 💡 Why Restaurant

- 💶 **Priced from the menu.** A line takes the price of the outlet's active menu when it is taken, and keeps it. A dish that is not on the menu, or not available today, is refused.
- 🔥 **The kitchen by station.** Sending an order makes one ticket for each station (grill, kitchen, bar, pastry) with only the new lines.
- 🧾 **Bills are ERP invoices.** One bill, or one for each seat, paid at the table by cash or card, with a tip owed to the staff.
- 📦 **Stock follows the plate.** Billing takes what each dish uses from the outlet's store, by its recipe, at the cost of goods sold.
- 🏨 **A bill can go on a hotel room.** With Hospitality installed, a guest's bill becomes a room charge on their folio.
- 🏠 **Self-hosted and open source.** AGPL-3.0, your server and your data, with no per-terminal fee.
- 🧩 **Extendable in Studio, open to AI agents.** Add fields to any record, and use the same records over REST and MCP under the same permissions.

## 🎯 At a glance

| | |
|---|---|
| **Version** | 0.3.3 |
| **Kind** | Its own interface at `/restaurant/`: the floor and the kitchen. Orders, menus, tables, reservations and outlets open in the platform's records screens |
| **What it adds** | 11 entities (6 you work with, 5 that sit inside them), 3 roles, 1 menu module, 1 package. It has no reports or print formats of its own |
| **Depends on** | Ferrum platform `>=0.3.2, <0.4` and [ERP](https://github.com/CBFerrum/ferrum-app-erp) `>=0.1` (selling, accounting, inventory and manufacturing) |
| **Works with** | [Hospitality](https://github.com/CBFerrum/ferrum-app-hospitality) (a bill on a guest's room), ERP (customers, items, recipes, invoices, payments, stock) |
| **Languages** | English, Arabic (right to left), Hindi |
| **Tests** | 1 Rust test (an end-to-end evening), 6 interface tests |
| **Licence** | AGPL-3.0-only |

## 📸 Screenshots

<p align="center"><picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-home-dark.webp"><img alt="Restaurant — home" src="docs/assets/screens/restaurant-home-light.webp"></picture><br><b>🍽️ The floor</b><br><sub>See each table, free or taken with its open order, the takeaways and deliveries still open, and the bills still to pay.</sub></p>

<table>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-outlet-list-dark.webp"><img alt="Outlets — list" src="docs/assets/screens/restaurant-outlet-list-light.webp"></picture>
<p><b>📍 Outlets</b><br><sub>Each restaurant with its menu, store and the accounts its takings go to.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-dining-table-list-dark.webp"><img alt="Tables — list" src="docs/assets/screens/restaurant-dining-table-list-light.webp"></picture>
<p><b>🪑 Tables</b><br><sub>Tables by outlet, with seats and whether an order holds them.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-menu-list-dark.webp"><img alt="Menus — list" src="docs/assets/screens/restaurant-menu-list-light.webp"></picture>
<p><b>📖 Menus</b><br><sub>Menus by outlet, each with its dishes and choices.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-menu-detail-dark.webp"><img alt="Menu — open record" src="docs/assets/screens/restaurant-menu-detail-light.webp"></picture>
<p><b>🥘 A menu</b><br><sub>Dishes with price, station and availability, and the choices they are ordered with.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-kitchen-dark.webp"><img alt="Restaurant — the kitchen" src="docs/assets/screens/restaurant-kitchen-light.webp"></picture>
<p><b>👨‍🍳 The kitchen</b><br><sub>Tickets by station: start each one, mark it ready, then served.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-order-detail-dark.webp"><img alt="Order — open record" src="docs/assets/screens/restaurant-order-detail-light.webp"></picture>
<p><b>📝 An order</b><br><sub>Take an order, send it to the kitchen, bill it and take payment.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-order-list-dark.webp"><img alt="Restaurant — orders" src="docs/assets/screens/restaurant-order-list-light.webp"></picture>
<p><b>🧾 Orders</b><br><sub>The day's orders, dine-in, takeaway and delivery, as a list or a board.</sub></p>
</td>
<td width="50%"></td>
</tr>
</table>

<p align="center">
<span><picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-home-phone-dark.webp"><img alt="Restaurant — home" src="docs/assets/screens/restaurant-home-phone-light.webp" width="260"></picture><br><sub><b>🍽️ The floor on a phone</b></sub></span>&nbsp;
<span><picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/restaurant-outlet-list-phone-dark.webp"><img alt="Outlets — list" src="docs/assets/screens/restaurant-outlet-list-phone-light.webp" width="260"></picture><br><sub><b>📍 Outlets on a phone</b></sub></span>&nbsp;
</p>

## ✨ Features

### 📍 Outlets, tables and menus

- **Outlets.** A restaurant with its company, a walk-in customer for bills that name nobody, an active menu, a store, and the cash, card and tips accounts its takings go to.
- **Tables.** A table belongs to an outlet and has a number of seats. It shows as taken while an order holds it.
- **Menus.** A menu belongs to an outlet and lists its dishes. Each dish is an ERP item with a price, a station (Kitchen, Grill, Bar or Pastry) and an available switch.
- **Choices (modifiers).** A menu lists the modifiers its dishes are ordered with (Extra Cheese, No Onion), each for one dish or every dish, with a price on top and optionally an item and quantity that stock takes.

### 📝 Orders

- **Three kinds.** Dine in (at a table), takeaway and delivery.
- **The floor starts an order.** A free table opens a new order for that table and its outlet. An empty restaurant says what to set up first.
- **One open order to a table.** The table shows as taken until the order is billed or cancelled.
- **Lines.** Dish, quantity, seat, choices and a note. A line has a state (New, Sent, Ready, Served, Void) that the kitchen moves on.
- **Choices are priced with the line,** written as the menu writes them and sent to the kitchen with the dish. A new line whose dish or choices change is priced again. Once sent, they are kept.
- **Guests, customer and contact.** The number of guests, an optional ERP customer, or a name and phone.
- **Delivery.** A delivery needs an address and a phone. Out for Delivery is allowed once the kitchen has made each dish, then Delivered.
- **Move to a Table.** An open dine-in order moves to another table of its outlet, with its kitchen tickets. Moved to a table with an open order, its dishes and guests join that order, and this one closes as Merged, pointing at it. Its tickets carry on for the order it joined.
- **Cancel the Order** while it is open and nothing has gone to the kitchen.

### 🔥 The kitchen

- **Send to the Kitchen** makes one ticket for each station with the new lines.
- **Ticket states.** Queued, Preparing (Start It), Ready, Served, or Voided when nothing is left to make. The order's lines follow the ticket.
- **The kitchen page** lists tickets that are not yet served, oldest first, with the table or the takeaway number, quantities and the choices a dish was ordered with. It refreshes every 10 seconds.
- **What went to the kitchen is voided, not edited.** A sent line cannot be changed or removed. It is voided and the dish ordered again. A voided line is neither charged nor billed, it is struck off its ticket, and a ticket left with nothing to make is Voided.
- **A line's state is the kitchen's.** A person only voids it.
- **An order with something sent cannot be cancelled.** Bill it, or void those lines first.

### 🧾 Billing and payment

- **Bill It** makes a draft ERP sales invoice of the order's lines, to its customer or the outlet's walk-in customer. **By seat** makes one bill for each seat's dishes. The order closes and the table is free.
- **Take Payment** pays one of the order's bills by **Cash**, **Card** or **Room**. It can pay all or part of a bill (a bill shared by two guests). The invoice is submitted if it is still a draft, and the payment is a payment entry against it, into the outlet's cash or card account.
- **Tips.** A tip on top is credited to the outlet's tips account, owed to the staff. Paying it out is payroll's.
- **Settled.** When every bill is paid, the order is Settled. The floor lists the bills still to be paid.
- **Paid by Room.** With Hospitality, a bill goes on a guest's room whole, without a tip and not after part of it was paid here. The room and the guest's name are checked against the stay, the bill becomes a room charge on the folio, and the bill's own draft invoice is dropped. It needs the hotel's Outlet Cashier right. The outlet names its hotel (Hotel Code) and the item the hotel invoices it as.
- **A card payment is recorded, not taken.** There is no card terminal or gateway yet.

### 📦 Stock by recipe

- **With a store,** billing takes what the dishes use from it in one submitted Material Issue at the cost of goods sold.
- **What a dish uses** is its item's bill of materials in ERP (scrap included), or the item itself when it is a stock item with no recipe (a bottled drink), plus what its choices add or leave out.
- **A store that runs short bills nothing** until it holds the stock, unless the company allows negative stock.

### 🪑 Reservations

- **A table held for a party at a time:** outlet, table, guest name, phone, party size, start and end, and a state (Booked, Seated, Cancelled, No Show).
- **A table holds one party at a time,** and no more than it seats. A cancelled reservation frees the time.
- **Staff enter them.** There is no public reservation page yet.

### 🔐 Roles, privacy and APIs

- **Three roles,** below. Row-level security applies to every table.
- **Personal data.** A request finds reservations and orders by phone number and erases the guest's name, phone and the delivery address.
- **Records for free.** Lists (and boards), forms, search, exports, audit trail, REST and MCP.
- **The assistant** knows what the records mean and suggests questions such as which tables are taken now and what sold best this week.

### 🌍 Languages

- **English, Arabic and Hindi** for the app's words and the names of its records. Pages mirror right to left. The Arabic and Hindi text was translated by a model and is waiting for review by native speakers.

## 🧭 How it works

```mermaid
flowchart LR
    T["Table or takeaway order"] --> S["Send to the Kitchen"]
    S --> K["Tickets by station"]
    K --> RD["Ready, then Served"]
    T --> B["Bill It: one or by seat"]
    B --> P["Take Payment: cash, card or room"]
    P --> ST["Settled"]
    B --> M["Material Issue from the store"]
```

A line is priced from the menu when it is taken. Sending makes the kitchen tickets, and billing makes the invoices and takes the dishes' stock. Payments are payment entries in ERP.

```mermaid
flowchart LR
    R["Restaurant"] --> ERP["ERP: customers, items, recipes, invoices, payments, stock"]
    R --> HOS["Hospitality: room charge on a folio"]
    R --> OB["Outbox: webhooks and notifications"]
```

Everything Restaurant adds is ordinary Ferrum metadata and Rust, so its records get permissions, an audit trail, exports and the REST and MCP APIs from the platform.

## 🧱 Data model

| Entity | What it is | Key fields or states |
|---|---|---|
| **Outlet** | A restaurant | Company, walk-in customer, menu, store, cash, card and tips accounts, hotel code, room charge item |
| **Dining Table** | A table | Outlet, seats, occupied |
| **Menu** | A priced menu for an outlet | Dishes, modifiers |
| **Restaurant Order** | Dine in, takeaway or delivery | Outlet, type, table, guests, customer, status (Open, Billed, Settled, Cancelled, Merged), delivery status, lines, total, bills |
| **Kitchen Ticket** | What one station makes | Order, station, status (Queued, Preparing, Ready, Served, Voided), items |
| **Table Reservation** | A table held for a party | Outlet, table, guest, party, start, end, status (Booked, Seated, Cancelled, No Show) |
| **Menu Dish, Menu Modifier, Order Line, Order Bill, Ticket Item** | Rows inside the records above | Dish and price, choice and price, line state, bill status (Unpaid, Partly Paid, Paid, On Room), ticket line |

| Role | What it can do |
|---|---|
| **Restaurant Manager** | Runs the outlets, tables and menus, and sees every order, ticket and reservation |
| **Server** | Takes orders and reservations, sends them to the kitchen and bills them, and takes payment |
| **Kitchen** | Works the tickets for their station, from preparing to served |

Actions on an order: Send to the Kitchen, Bill It, Move to a Table, Take Payment, Cancel the Order, Out for Delivery, Delivered. On a ticket: Start It, Ready, Served.

## 🔌 Integrations

| With | How | Needs |
|---|---|---|
| **ERP** | Customers, items (dishes), bills of materials, draft sales invoices, payment entries, material issues | ERP installed (done for you) |
| **Hospitality** | A bill goes on a guest's room as a room charge | Hospitality installed, the Outlet Cashier right, a hotel code on the outlet |
| **Webhooks and the outbox** | Record events from the platform | An endpoint |
| **REST and MCP** | Every record and action, within each person's permissions | An API token |
| **Personal-data requests** | Export and erase a guest's data by phone | None |

Card terminals, payment gateways, delivery platforms and kitchen printers are not connected yet.

## 📊 How it compares

Leaders: Toast, Square for Restaurants, Lightspeed Restaurant, TouchBistro and Odoo POS Restaurant (open source).

| Capability | Ferrum Restaurant | Toast | Square | Lightspeed | TouchBistro | Odoo POS |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| Self-hosted, you hold the data | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Open-source licence | ✅ | ❌ | ❌ | ❌ | ❌ | 🟡¹ |
| No per-terminal or monthly licence fee | ✅ | ❌ | ❌ | ❌ | ❌ | 🟡² |
| Sales, stock and accounts are one data set | ✅ | 🟡³ | 🟡³ | 🟡³ | 🟡³ | ✅ |
| Menu pricing with modifiers priced on top | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Modifier groups: required, min and max, sizes | 🟡⁴ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| Combos and set menus | ❌ | ✅ | 🟡 | ✅ | 🟡 | ✅ |
| Touch order screen for servers | ❌⁵ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Floor plan with a layout | 🟡⁶ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Move and merge tables | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Kitchen tickets by station | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Kitchen display with timers and bump | 🟡⁷ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Kitchen and bar printers | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Courses and fire or hold | ❌ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| Voiding a sent dish, struck off its ticket | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Discounts, comps and voids with a reason and manager approval | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Split a bill by seat or equally | 🟡⁸ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Tips, with pooling and payout | 🟡⁹ | ✅ | ✅ | 🟡 | ✅ | 🟡 |
| Integrated card terminal | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Stock taken by recipe when billing | ✅ | ✅ | 🟡 | ✅ | ✅ | ✅ |
| Reservations | 🟡¹⁰ | ✅ | 🟡 | 🟡 | ✅ | 🟡 |
| Online ordering and delivery platforms | ❌ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| QR ordering and pay at the table | ❌ | ✅ | ✅ | ✅ | 🟡 | ✅ |
| A bill on a hotel guest's room | ✅ | 🟡¹¹ | 🟡¹¹ | 🟡¹¹ | 🟡¹¹ | ❌ |
| Loyalty and gift cards | ❌ | ✅ | ✅ | 🟡 | ✅ | ✅ |
| Reports by item, server, daypart and voids | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Works offline | ❌ | ✅ | ✅ | ✅ | ✅ | 🟡 |

<sub>¹ Odoo's Community edition is open source; some features are Enterprise only. ² Community is free; Enterprise is paid per user. ³ By accounting and inventory integrations. ⁴ A flat list of choices typed as comma-separated text. ⁵ An order is taken on the records form, not on a tile screen. ⁶ A tile per table, free or taken; no sections or drag layout. ⁷ A page of tickets by station with Start, Ready and Served; no timers or bump. ⁸ By seat, or one bill paid in parts; no equal split. ⁹ A tip is credited to a liability account; paying it out is payroll's. ¹⁰ Staff enter them; no online page, reminders or deposits. ¹¹ Through a hotel system integration.</sub>

### 🏆 Where Restaurant stands out

- 🧾 The bill is an ERP invoice, the payment a payment entry and the stock issue a material issue, so the till and the books never disagree.
- 📦 Stock goes out by recipe at billing, and a store that runs short stops the bill.
- 🔥 A sent dish is voided, never silently edited, and the kitchen's ticket shows it struck off.
- 🏨 A restaurant bill goes on a hotel room through Hospitality.
- 🏠 Self-hosted, open source, with Arabic and Hindi and right-to-left pages.

### 🚧 Where the leaders are still ahead

- 📱 **Taking the order.** A touch screen of dish tiles on a handheld, with modifier groups, combos and courses.
- 🖨️ **Hardware.** Kitchen and bar printers, kitchen displays with timers, card terminals and cash drawers.
- 🛵 **Channels.** Online ordering, QR ordering and delivery platforms.
- 💳 **Paying.** Equal splits, discounts and comps with approval, service charge, tip pooling.
- 📊 **Reports.** Sales by item, server and daypart, voids, covers and table turn time.
- 📴 **Offline** operation and a floor plan with sections.

> [!NOTE]
> Leader columns are from public product information as of November 2026 and may change. Features marked 🟡 for a leader depend on plan, region or add-on.

## 📍 Roadmap

### ✅ Recently shipped

- [x] **0.3.3:** the interface on the tuned design system, with better menu icons and cleaner print.
- [x] **0.3.2:** the redesigned interface, with the floor and the kitchen on the work-home pattern.
- [x] **0.3.1:** a bill on a hotel guest's room, through Hospitality's room charges.
- [x] **0.3.0:** choices priced on top of their dish, orders moved and merged between tables, bills by seat paid at the table with tips, and dishes' recipes taken from the store.
- [x] **0.2.0:** a voided dish struck off its kitchen ticket, a line's state moved only by the kitchen, a delivery that goes out once made.

### 🔜 Next up

**Taking orders**
- [ ] A touch order screen: dish tiles, modifiers, seat, course and send. Today an order is a records form.
- [ ] Modifier groups: required, minimum and maximum, single or multiple, sizes, half and half.
- [ ] Combos and set menus, and portions and sizes as variants of a dish.
- [ ] Courses and fire or hold control.

**Kitchen**
- [ ] A kitchen display with age timers, colours, bump, recall and sound, pushed live.
- [ ] Kitchen and bar printers, with routing by dish and order type, and custom stations.
- [ ] Sold out from stock: hide a dish when the store lacks it.

**Paying**
- [ ] Discounts, comps and voids with a reason and manager approval. The bill's discount is fixed at zero today.
- [ ] Equal split, split by item and split by amount.
- [ ] Service charge and automatic gratuity for large parties.
- [ ] Card payment through a terminal or gateway, and a tab held with a card.
- [ ] Payment through the Point of Sale till, with its shifts, drawer and Z report, instead of straight to the outlet's accounts.

### 🔭 Later

- [ ] A floor plan with sections, a drag layout and table timers.
- [ ] Daypart menus (breakfast, lunch, dinner) switching by time.
- [ ] A server on each order, with a server sales and tips report, and tip pooling and payout.
- [ ] Allergens, diet tags and pictures on menu items.
- [ ] QR ordering and pay at the table.
- [ ] Online ordering and delivery platforms (for example Swiggy, Zomato, ONDC), with menu sync.
- [ ] Delivery zones, fees, minimums and rider assignment.
- [ ] A public reservation page with a waiting list, reminders and deposits.
- [ ] Loyalty for guests, through the Point of Sale programme.
- [ ] Reports: sales by item, category, server and daypart, voids, covers and table turn time.
- [ ] Recipe cost and margin per dish, waste and spoilage, theoretical against actual usage.
- [ ] Offline operation.

## 🚀 Install

Restaurant is a native Ferrum app. It ships in the official Ferrum server image, and a workspace administrator installs it from **Apps** (`/store/`). It requires ERP `>=0.1` (with its selling, accounting, inventory and manufacturing), and installing it in a workspace without ERP installs ERP whole. Self-hosters who build their own image enable the `restaurant` feature of the server.

First steps after installing:

1. Give people a role: **Restaurant Manager**, **Server** or **Kitchen**.
2. In ERP, make the dishes as items. Give recipe dishes a bill of materials.
3. Create an **Outlet** with its walk-in customer, store, and the cash, card and tips accounts of its company.
4. Add **Tables** with their seats, and a **Menu** with dishes, prices, stations and choices. Set it as the outlet's menu.
5. Open **The floor** and start an order from a free table.
6. To put bills on hotel rooms, install Hospitality, give the cashier its **Outlet Cashier** role, and fill the outlet's Hotel Code and room charge item.

## 🔧 Develop

Clone it next to the platform:

```text
work/
├── ferrum/                  https://github.com/CBFerrum/Ferrum (with Ferrum-Frontend cloned into ferrum/web)
└── ferrum-app-restaurant/
```

Point the platform dependencies at your checkout (this file is git-ignored):

```toml
# .cargo/config.toml
[patch."https://github.com/CBFerrum/Ferrum"]
ferrum-domain = { path = "../ferrum/crates/domain" }
ferrum-kernel = { path = "../ferrum/crates/kernel" }
ferrum-meta = { path = "../ferrum/crates/meta" }
```

The tests need PostgreSQL with an ordinary (non-superuser) role, so row-level security applies:

```bash
createuser ferrum --pwprompt && createdb -O ferrum ferrum_restaurant_test
export FERRUM_RESTAURANT_TEST_DATABASE_URL=postgres://ferrum:ferrum@localhost:5432/ferrum_restaurant_test
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Its tests also build on [ferrum-app-erp](https://github.com/CBFerrum/ferrum-app-erp) and [ferrum-app-hospitality](https://github.com/CBFerrum/ferrum-app-hospitality) (for a bill put on a room), which Cargo takes from GitHub. The interface (`ui/`) is built with the platform's packages from the sibling `ferrum/web` checkout, and its built files (`ui/dist`) are committed and embedded in the crate:

```bash
cd ui
npm run build && npm run check && npm run typecheck && npm test
```

The end-to-end test `tests/restaurant_it.rs` walks an evening: a table's order through the kitchen to the bill, a voided line, a takeaway and a delivery, choices priced and sent to the kitchen, a party moved to a bigger table and another merged into it, bills by seat paid by card with a tip and by cash in parts, the dishes' recipes taken from the store and a store that runs short, reservations, and who may do what.

## 📂 Project layout

```text
app.yaml          the manifest: id, version, requirements, what it adds to Ferrum
packages/         metadata: entities, roles, menu, a package guide
src/              the logic in Rust: orders, kitchen, bills, reservations
ui/               its interface: the floor and the kitchen (React, TypeScript)
tests/            an end-to-end test against a real PostgreSQL
docs/PROVENANCE.md  what the design took from ERPNext 12's Restaurant module
CHANGELOG.md      what changed in each version
```

Outlets, tables, menus and reservations are adapted as designs from ERPNext 12's Restaurant module (GPL-3.0); see [docs/PROVENANCE.md](docs/PROVENANCE.md).

## 📜 Changelog

Every version is in [CHANGELOG.md](CHANGELOG.md).

## 🤝 Contributing

Issues and pull requests are welcome at [github.com/CBFerrum/ferrum-app-restaurant](https://github.com/CBFerrum/ferrum-app-restaurant/issues). Run the checks in Develop first.

## 📄 License

[AGPL-3.0-only](LICENSE), like the platform.

<div align="center">

<sub>Part of <a href="https://github.com/CBFerrum/Ferrum">Ferrum</a>, the open business platform · Made with 🦀 Rust and 🐘 PostgreSQL</sub>

</div>
