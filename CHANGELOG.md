# Changelog

## 0.2.1

Rebuilt with Ferrum-Frontend a64416a: a new record's box whose default is 1 shows ticked, as it is saved. An action's dialog says every refusal, the errors on what the action made (another record's rows) among them, and an action with parameters and a confirmation asks once.

## 0.2.0

What walking an evening through the pages found:
- A line voided after it went to the kitchen stayed on its ticket, so the kitchen would still make it. It is now struck off the ticket, and a ticket left with nothing to make is Voided and leaves the kitchen's page.
- A line's state could be set by hand: a sent dish back to New, to be sent again, or Served without the kitchen. Only the kitchen moves a line on now; a person voids it.
- A delivery could go out before its food had gone to the kitchen. It goes out once each dish is ready or served.
- A free table on the floor opened an order with nothing filled in. It opens one for that table and its outlet. The floor starts a takeaway or a delivery too, and an empty restaurant says what to set up first.
- A dish typed in another case kept it on the ticket and the bill. It is written as the menu writes it.
- Actions said only "done". Each says what it did: where an order was sent, what it was billed, a ticket's step.
- A ticket for "Table 1" read "Table Table 1". An order was named by its type ("Dine In") wherever it was linked. It is named by its number, and is found by its guest's name or phone.
- On an order's page, a sent line's dish, quantity and note are read-only, a table's fields show for dine-in only, and a delivery's for deliveries.
- A reservation ending before it starts says it "must be after it starts".

## 0.1.7

Rebuilt with Ferrum-Frontend a76295e: its pages show times on a person's own clock when they chose one on their Account page, while today and due dates stay on the workspace's day.

## 0.1.6

Rebuilt with Ferrum-Frontend 24c3951: its pages show times on the workspace's clock, naming the zone when it is not the browser's, a draft numbered on submit is headed as a draft, and a record of an app that is not installed says so.

## 0.1.5

Rebuilt with Ferrum-Frontend bb60115: a record page reads an empty number or check as the server does, so an action whose condition needs one shows, and a workspace's settings open as their one record.

## 0.1.4

Rebuilt with Ferrum-Frontend 00dea07: in a workflow state that names what may change, the other fields show read-only, and the fields the state needs show required.

## 0.1.3

Personal-data requests find a guest's table reservations and orders by their phone number, however written, or list them by name; erasure blanks the name, number and delivery address, and the bills keep what they carry (Ferrum's FERRUM-PRIVACY-002).

## 0.1.2

Rebuilt with Ferrum-Frontend daeadda: after an action runs, a record page shows what the action said it did rather than only that it is done.

## 0.1.1

Rebuilt with Ferrum-Frontend 80ff179: a field's label names its control for assistive technology, and a record whose title is a link shows that record's title, not its id.

## 0.1.0

A restaurant (FERRUM-INDUSTRY-002).

- Outlets, tables and menus with dishes by station.
- Orders for dine-in, takeaway and delivery, priced from the menu when taken; one open order to a
  table; sent lines voided rather than edited.
- Kitchen tickets by station, from queued to served, the order's lines following.
- Bills through ERP to the order's customer or the walk-in customer.
- Table reservations within the seats, one party at a time.
- Its own interface at `/restaurant/`: the floor and the kitchen.
