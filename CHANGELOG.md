# Changelog

## 0.3.2

The interface follows the upgraded design system, rebuilt with Ferrum-Frontend 0a47157. Text uses the kit's type roles (nothing under 12px) and controls its height tokens; loading shows skeletons shaped like what is coming; empty, no matches, denied, offline and failed are told apart; status colours come from stored state ids. The interface check now runs the UI lint against an empty baseline. The home is rebuilt on the kit's work-home pattern: one page title, the main action as a button, queues with their counts, status at its natural width; its other own pages follow the same anatomy.

## 0.3.1

A bill on a hotel guest's room (FEO-045). Where Hospitality is installed too, Take Payment may put a bill on a room (Paid By Room, with the room and the guest's name): it becomes one of Hospitality's room charges, checked against the guest in the room and posted to their folio, so the hotel invoices it with the room as the outlet's Room Charge Item, and the bill's own draft invoice is dropped. Putting it there is Hospitality's Outlet Cashier's right. A bill goes on a room whole, without a tip, and not after part of it was paid here. The outlet names its hotel (Hotel Code). The order is Settled once every bill is paid or on a room.

## 0.3.0

Restaurant service and stock on ERP's stock and payment owners (FEO-046).

- **Choices.** A menu lists modifiers (Extra Cheese, No Onion), each for one dish or every dish, with a price on top. A line's choices are priced with it, written as the menu writes them and sent to the kitchen with the dish; a new line whose dish or choices change is priced again, and once sent they are kept. The kitchen page shows them.
- **Moved and merged.** Move to a Table takes an open order to another table of its outlet. A table with an open order takes its dishes and guests, and this one closes as Merged, pointing at it. Its kitchen tickets follow, their lines re-pointed so the kitchen still moves them on.
- **Bills by seat.** Bill It makes one invoice, or one for each seat (every line then has its seat). The order keeps its bills.
- **Stock by recipe.** With a store, billing takes what the dishes use from it as one submitted Material Issue at the cost of goods sold: each dish's recipe (its item's bill of materials in ERP, scrap included), or a stock item sold as it is, with what its choices add or leave out. A store that runs short bills nothing.
- **Paid at the table.** Take Payment pays a bill by cash or card through a submitted ERP payment entry (a draft invoice is submitted first), in full or in part, with a tip credited to the outlet's tips account. Every bill paid, the order is Settled. The floor lists the bills still to pay.
- The outlet has a store, cash, card and tips accounts, all its company's. The package needs ERP's inventory and manufacturing too. Names in Arabic and Hindi for the new fields, and Table, Order, Open, Server, Bar and Party corrected (they were translated in their computing or legal sense).

## 0.2.7

Rebuilt with Ferrum-Frontend c59c9a2: the names of its models, fields and manifest in Arabic and Hindi (100 each in ui/src/locales/names, which the platform serves to every page of the workspace, so a record's labels, the launcher and the bar read in the person's language). Translated by the model, to be reviewed by native speakers.

## 0.2.6

The assistant knows what its records mean in its pages (contributes.shell.assistant): their states, what counts and where figures come from, with questions it suggests asking there.

## 0.2.5

Rebuilt with Ferrum-Frontend 5b22e7b: its own words carry built-in Arabic and Hindi (20 words each, ui/src/locales, translated by the model and to be reviewed by native speakers), fetched for the page's language only. The labels it keeps in objects, its dialogs, its page titles and the words it keeps in variables go through t() as well, and the names and sizes it shows as they are keep their order right to left. Its pages are drawn once their words are in, and its entry carries only the parts of the kit it uses.

## 0.2.4

Rebuilt with Ferrum-Frontend 6537d65: an action's link, code or token shown once is kept in a dialog to copy, and on a page that reads right to left, English left untranslated keeps its own order. Its own words go through the kit's t() (15 places), so a workspace's own words for its language reach them, and Admin, Interface words lists them under it (dist/words.json).

## 0.2.3

Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own words for its interface (Admin, Interface words) reach its pages as they load; an attachment field that holds a picture shows it; and a value another record already has is said so, where the form said the record had been changed by someone else.

## 0.2.2

Rebuilt with Ferrum-Frontend 63280d3: right to left, its pages mirror as they read (icons that point along the line, margins and alignment by the line's start and end, arrow keys along it), and the records pages' words are in Arabic and Hindi.

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
