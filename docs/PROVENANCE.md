# Provenance

## ERPNext 12's Restaurant module

- Source: [frappe/erpnext](https://github.com/frappe/erpnext), branch `version-12`, commit d6da1a9,
  `erpnext/restaurant/doctype/`.
- Licence: GNU General Public License v3. This app is under the GNU Affero General Public License v3,
  which may be combined with GPL-3.0 code (section 13 of each licence).
- What was taken, as designs rather than code:
  - its doctypes Restaurant, Restaurant Table, Restaurant Menu (with its items) and Restaurant
    Reservation, adapted as Outlet, Dining Table, Menu and Table Reservation;
  - Restaurant Order Entry's way of keeping an order by table and invoicing it, adapted as Restaurant
    Order;
  - the rule that a line is priced from the restaurant's active menu.
- Its own: kitchen stations and tickets, voiding sent lines, takeaway and delivery, the reservation
  checks, and all the code.

The copy read is in `ERP/referance/industry/erpnext-v12` on the machine this was written on (sparse:
`erpnext/hotels` and `erpnext/restaurant`).
