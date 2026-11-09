//! An evening at a restaurant (FERRUM-INDUSTRY-002) against a real PostgreSQL
//! (`FERRUM_RESTAURANT_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_restaurant_test`;
//! reset first): a table's order priced from the menu, one open order to a table, its lines sent to
//! the kitchen as a ticket for each station and worked through to served, sent lines voided rather
//! than edited, the bill made through ERP and the table freed; a takeaway and a delivery; choices
//! priced on top of their dish; a party moved to another table and another merged into it; bills
//! by seat, paid at the table by card and cash with a tip; the dishes' recipes taken from the
//! store; table reservations that hold one party at a time within the seats; and who may do what.

use ferrum_kernel::privacy::Person;
use ferrum_kernel::secret::SecretKey;
use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

fn dec(v: &Value) -> f64 {
    v.as_str().map(|s| s.parse().unwrap()).or_else(|| v.as_f64()).unwrap_or_else(|| panic!("not a number: {v}"))
}

struct T {
    k: Arc<Kernel>,
    tenant: Uuid,
    admin: Ctx,
}

impl T {
    async fn insert(&self, ctx: &Ctx, entity: &str, v: Value) -> Doc {
        self.k.insert(ctx, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"))
    }
    async fn get(&self, entity: &str, id: Uuid) -> Doc {
        self.k.get(&self.admin, entity, id, false).await.unwrap_or_else(|e| panic!("get {entity}: {e:?}"))
    }
    async fn find(&self, entity: &str, filters: Value) -> Vec<Doc> {
        let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), limit: Some(50), ..Default::default() };
        self.k.list(&self.admin, entity, &q).await.unwrap().rows
    }
    async fn find_one(&self, entity: &str, filters: Value) -> Doc {
        let rows = self.find(entity, filters.clone()).await;
        assert_eq!(rows.len(), 1, "{entity} {filters}");
        rows[0].clone()
    }
    async fn user(&self, email: &str, roles: &[&str]) -> Ctx {
        let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
        let id = self.k.create_user(&self.admin, email, email, Some("Correct-Horse-9"), &roles, false).await.expect("user");
        let s = self.k.login("tandoor", email, "Correct-Horse-9").await.expect("login");
        self.k.load_ctx(self.tenant, id, Some(s.token_version), "it".into()).await.expect("ctx")
    }
}

fn refused<V: std::fmt::Debug>(r: Result<V, Error>, kind: &str, says: &str) {
    let msg = format!("{r:?}");
    let ok = matches!(
        (&r, kind),
        (Err(Error::Invalid(_) | Error::Validation(_)), "invalid") | (Err(Error::Forbidden(_)), "forbidden") | (Err(Error::NotFound(_)), "not found")
    );
    assert!(ok && msg.contains(says), "expected {kind} saying {says:?}, got {msg}");
}

fn lines(order: &Doc) -> Vec<(String, i64, String)> {
    order["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (l["dish"].as_str().unwrap().into(), l["qty"].as_i64().unwrap(), l["state"].as_str().unwrap().into()))
        .collect()
}

#[test]
fn an_evening() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

async fn scenario() {
    let url = std::env::var("FERRUM_RESTAURANT_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_restaurant_test".into());
    let db = Db::connect(DbConfig { url, max_connections: 16, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
    packages.extend(ferrum_app_restaurant::packages().expect("app packages"));
    packages.extend(ferrum_app_hospitality::packages().expect("Hospitality's packages"));
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    ferrum_app_erp::register(&mut hooks);
    ferrum_app_restaurant::register(&mut hooks);
    ferrum_app_hospitality::register(&mut hooks);
    let settings = KernelSettings { secret_key: Some(SecretKey::new([7u8; 32])), ..Default::default() };
    let k = Kernel::boot(db, catalog, hooks, settings, None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    ferrum_app_erp::maintenance(&k).await.expect("partitions");
    let tenant = k
        .create_tenant(NewTenant {
            slug: "tandoor".into(),
            name: "The Tandoor".into(),
            admin_email: "admin@tandoor.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["restaurant".into(), "hospitality".into()],
            settings: Some(json!({"fiscal_year_start_month": 4, "timezone": "Asia/Kolkata"})),
        })
        .await
        .expect("tenant");
    let session = k.login("tandoor", "admin@tandoor.test", "Correct-Horse-9").await.expect("login");
    let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    let t = T { k: k.clone(), tenant, admin };
    let a = t.admin.clone();

    // ───────────── the books and the menu ─────────────
    let inr = t.find_one("Currency", json!({"code": "INR"})).await;
    let company = uid(&t.insert(&a, "Company", json!({"company_name": "The Tandoor", "abbr": "TT", "default_currency": inr["id"]})).await["id"]);
    let services = t.find_one("ItemGroup", json!({"group_name": "Services"})).await;
    let nos = t.find_one("Uom", json!({"uom_name": "Nos"})).await;
    let item = |code: &str| json!({"item_code": code, "item_name": code, "item_group": services["id"], "stock_uom": nos["id"], "is_stock_item": false});
    let tikka = uid(&t.insert(&a, "Item", item("TIKKA")).await["id"]);
    let dal = uid(&t.insert(&a, "Item", item("DAL")).await["id"]);
    let soda = uid(&t.insert(&a, "Item", item("SODA")).await["id"]);
    let jamun = uid(&t.insert(&a, "Item", item("JAMUN")).await["id"]);
    let walk_in = uid(&t.insert(&a, "Customer", json!({"customer_name": "Walk-in", "customer_type": "Individual"})).await["id"]);
    let manager = t.user("manager@tandoor.test", &["Restaurant Manager"]).await;
    let server = t.user("server@tandoor.test", &["Server"]).await;
    let cook = t.user("cook@tandoor.test", &["Kitchen"]).await;
    let outlet = uid(&t
        .insert(&manager, "Outlet", json!({"outlet_name": "The Tandoor", "company": company.to_string(), "walk_in_customer": walk_in.to_string()}))
        .await["id"]);
    let menu = uid(&t
        .insert(
            &manager,
            "Menu",
            json!({"menu_name": "Dinner", "outlet": outlet.to_string(), "dishes": [
                {"item": tikka.to_string(), "dish_name": "Paneer Tikka", "price": "320", "station": "Grill"},
                {"item": dal.to_string(), "dish_name": "Dal Makhani", "price": "220", "station": "Kitchen"},
                {"item": soda.to_string(), "dish_name": "Lime Soda", "price": "90", "station": "Bar"},
                {"item": jamun.to_string(), "dish_name": "Gulab Jamun", "price": "150", "station": "Pastry", "available": false}]}),
        )
        .await["id"]);
    t.k.update(&manager, "Outlet", outlet, obj(json!({"menu": menu.to_string()})), None).await.unwrap();
    let t1 = uid(&t.insert(&manager, "DiningTable", json!({"table_name": "1", "outlet": outlet.to_string(), "seats": 4})).await["id"]);
    let t2 = uid(&t.insert(&manager, "DiningTable", json!({"table_name": "2", "outlet": outlet.to_string(), "seats": 2})).await["id"]);

    // ───────────── a table's order ─────────────
    let order = |table: Option<Uuid>, kind: &str, dishes: &[(&str, i64)], extra: Value| {
        let mut v = json!({"outlet": outlet.to_string(), "order_type": kind, "table": table.map(|t| t.to_string()), "guests": 3,
                           "lines": dishes.iter().map(|(d, q)| json!({"dish": d, "qty": q})).collect::<Vec<_>>()});
        v.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap_or_default());
        obj(v)
    };
    refused(
        t.k.insert(&server, "RestaurantOrder", order(Some(t1), "Dine In", &[("Butter Naan", 2)], json!({}))).await,
        "invalid",
        "Butter Naan is not on the menu",
    );
    refused(t.k.insert(&server, "RestaurantOrder", order(Some(t1), "Dine In", &[("Gulab Jamun", 1)], json!({}))).await, "invalid", "not available today");
    refused(t.k.insert(&server, "RestaurantOrder", order(None, "Dine In", &[("Dal Makhani", 1)], json!({}))).await, "invalid", "needs its table");
    let first =
        t.k.insert(&server, "RestaurantOrder", order(Some(t1), "Dine In", &[("Paneer Tikka", 2), ("dal makhani", 1), ("Lime Soda", 2)], json!({})))
            .await
            .expect("an order");
    assert_eq!(dec(&first["total"]), 1040.0, "2 × 320 + 220 + 2 × 90, priced from the menu");
    assert!(first["lines"].as_array().unwrap().iter().any(|l| l["dish"] == "Dal Makhani"), "written as the menu writes it");
    let oid = uid(&first["id"]);
    assert!(t.get("DiningTable", t1).await["occupied"].as_bool().unwrap(), "the table is taken");
    refused(t.k.insert(&server, "RestaurantOrder", order(Some(t1), "Dine In", &[("Lime Soda", 1)], json!({}))).await, "invalid", "has an open order");
    refused(t.k.insert(&cook, "RestaurantOrder", order(Some(t2), "Dine In", &[("Lime Soda", 1)], json!({}))).await, "forbidden", "");

    // To the kitchen: a ticket for each station.
    let sent = t.k.run_action(&server, "RestaurantOrder", oid, "send_to_kitchen", Value::Null).await.expect("sent");
    assert_eq!(sent["tickets"].as_array().unwrap().len(), 3);
    assert_eq!(sent["message"], "Sent to Bar, Grill and Kitchen, for Table 1");
    let tickets = t.find("KitchenTicket", json!({"order": oid.to_string()})).await;
    let mut stations: Vec<String> = tickets.iter().map(|x| x["station"].as_str().unwrap().to_string()).collect();
    stations.sort();
    assert_eq!(stations, ["Bar", "Grill", "Kitchen"]);
    refused(t.k.run_action(&server, "RestaurantOrder", oid, "send_to_kitchen", Value::Null).await, "invalid", "nothing new");
    let now = t.get("RestaurantOrder", oid).await;
    assert!(lines(&now).iter().all(|(_, _, st)| st == "Sent"));
    // A line's state is the kitchen's to move on: a person only voids it.
    let mut by_hand = now["lines"].as_array().unwrap().clone();
    by_hand[0]["state"] = json!("Served");
    refused(t.k.update(&server, "RestaurantOrder", oid, obj(json!({"lines": by_hand})), None).await, "invalid", "the kitchen moves it on");
    let mut again = now["lines"].as_array().unwrap().clone();
    again[0]["state"] = json!("New");
    refused(t.k.update(&server, "RestaurantOrder", oid, obj(json!({"lines": again})), None).await, "invalid", "the kitchen moves it on");
    // A sent line is voided, not edited; another dish comes as a line of its own.
    let mut changed = now["lines"].as_array().unwrap().clone();
    changed[0]["qty"] = json!(3);
    refused(t.k.update(&server, "RestaurantOrder", oid, obj(json!({"lines": changed})), None).await, "invalid", "void it and order again");
    let mut renamed = now["lines"].as_array().unwrap().clone();
    renamed[0]["dish"] = json!("Dal Makhani");
    refused(t.k.update(&server, "RestaurantOrder", oid, obj(json!({"lines": renamed})), None).await, "invalid", "void it and order again");
    // The menu's price changes after the order was taken: what was ordered keeps its price.
    let mut dishes = t.get("Menu", menu).await["dishes"].as_array().unwrap().clone();
    for d in dishes.iter_mut().filter(|d| d["dish_name"] == "Paneer Tikka") {
        d["price"] = json!("350");
    }
    t.k.update(&manager, "Menu", menu, obj(json!({"dishes": dishes})), None).await.unwrap();
    let mut more = now["lines"].as_array().unwrap().clone();
    more.push(json!({"dish": "Dal Makhani", "qty": 1}));
    let more = t.k.update(&server, "RestaurantOrder", oid, obj(json!({"lines": more})), None).await.expect("another dal");
    assert_eq!(dec(&more["total"]), 1260.0);
    let mut voided = more["lines"].as_array().unwrap().clone();
    for l in voided.iter_mut().filter(|l| l["dish"] == "Lime Soda") {
        l["state"] = json!("Void");
    }
    let voided = t.k.update(&server, "RestaurantOrder", oid, obj(json!({"lines": voided})), None).await.expect("the sodas voided");
    assert_eq!(dec(&voided["total"]), 1080.0, "a voided line is not charged");
    // The bar is told: its ticket had only the sodas, so it has nothing left to make.
    let bar = t.get("KitchenTicket", uid(&t.find("KitchenTicket", json!({"order": oid.to_string(), "station": "Bar"})).await[0]["id"])).await;
    assert_eq!(bar["status"], "Voided", "{bar:?}");
    assert!(bar["items"].as_array().unwrap().iter().all(|i| i["voided"] == true), "{bar:?}");
    let grill_before = t.get("KitchenTicket", uid(&t.find("KitchenTicket", json!({"order": oid.to_string(), "station": "Grill"})).await[0]["id"])).await;
    assert_eq!(grill_before["status"], "Queued", "the grill's ticket is untouched");
    let sent = t.k.run_action(&server, "RestaurantOrder", oid, "send_to_kitchen", Value::Null).await.expect("the new dal sent");
    assert_eq!(sent["tickets"].as_array().unwrap().len(), 1);

    // The grill works its ticket; the order's tikka follows it.
    let grill = t.find("KitchenTicket", json!({"order": oid.to_string(), "station": "Grill"})).await[0].clone();
    let gid = uid(&grill["id"]);
    refused(t.k.run_action(&cook, "KitchenTicket", gid, "ready", Value::Null).await, "invalid", "not available");
    t.k.run_action(&cook, "KitchenTicket", gid, "start", Value::Null).await.unwrap();
    t.k.run_action(&cook, "KitchenTicket", gid, "ready", Value::Null).await.unwrap();
    assert!(lines(&t.get("RestaurantOrder", oid).await).iter().any(|(d, _, st)| d == "Paneer Tikka" && st == "Ready"));
    t.k.run_action(&cook, "KitchenTicket", gid, "served", Value::Null).await.unwrap();
    assert!(lines(&t.get("RestaurantOrder", oid).await).iter().any(|(d, _, st)| d == "Paneer Tikka" && st == "Served"));
    assert!(lines(&t.get("RestaurantOrder", oid).await).iter().any(|(d, _, st)| d == "Lime Soda" && st == "Void"), "a voided line stays voided");
    refused(t.k.run_action(&server, "RestaurantOrder", oid, "cancel_order", Value::Null).await, "invalid", "went to the kitchen");

    // The bill.
    let billed = t.k.run_action(&server, "RestaurantOrder", oid, "bill", Value::Null).await.expect("billed");
    assert!(billed["message"].as_str().unwrap().ends_with("a draft invoice in ERP; the table is free"), "{billed}");
    let inv = t.get("SalesInvoice", uid(&billed["sales_invoice"]["id"])).await;
    assert_eq!(inv["customer"].as_str(), Some(walk_in.to_string().as_str()));
    let mut billed_lines: Vec<(String, f64, f64)> =
        inv["items"].as_array().unwrap().iter().map(|l| (l["item"].as_str().unwrap().to_string(), dec(&l["qty"]), dec(&l["price_list_rate"]))).collect();
    billed_lines.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.total_cmp(&y.1)));
    let mut want: Vec<(String, f64, f64)> = vec![(tikka.to_string(), 2.0, 320.0), (dal.to_string(), 1.0, 220.0), (dal.to_string(), 1.0, 220.0)];
    want.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.total_cmp(&y.1)));
    assert_eq!(billed_lines, want, "every line but the voided sodas");
    assert!(!t.get("DiningTable", t1).await["occupied"].as_bool().unwrap(), "the table is free");
    refused(t.k.update(&server, "RestaurantOrder", oid, obj(json!({"guests": 4})), None).await, "invalid", "billed");
    t.k.insert(&server, "RestaurantOrder", order(Some(t1), "Dine In", &[("Lime Soda", 1)], json!({}))).await.expect("the table takes its next order");

    // A takeaway needs no table; a delivery needs somewhere to go.
    let away = uid(&t
        .insert(&server, "RestaurantOrder", json!({"outlet": outlet.to_string(), "order_type": "Takeaway", "lines": [{"dish": "Dal Makhani", "qty": 2}]}))
        .await["id"]);
    t.k.run_action(&server, "RestaurantOrder", away, "cancel_order", Value::Null).await.expect("not sent: it may be cancelled");
    refused(t.k.insert(&server, "RestaurantOrder", order(None, "Delivery", &[("Dal Makhani", 1)], json!({}))).await, "invalid", "address and a phone");
    let delivery = uid(&t
        .insert(
            &server,
            "RestaurantOrder",
            json!({"outlet": outlet.to_string(), "order_type": "Delivery", "customer_name": "Bo", "phone": "+91 98000 00000", "delivery_address": "4 Lake Road",
                                                       "lines": [{"dish": "Paneer Tikka", "qty": 1}]}),
        )
        .await["id"]);
    refused(t.k.run_action(&server, "RestaurantOrder", delivery, "delivered", Value::Null).await, "invalid", "not available");
    // It goes out with its food: made at the grill first.
    refused(t.k.run_action(&server, "RestaurantOrder", delivery, "dispatch", Value::Null).await, "invalid", "Paneer Tikka is not ready yet");
    t.k.run_action(&server, "RestaurantOrder", delivery, "send_to_kitchen", Value::Null).await.unwrap();
    let made = uid(&t.find("KitchenTicket", json!({"order": delivery.to_string()})).await[0]["id"]);
    t.k.run_action(&cook, "KitchenTicket", made, "start", Value::Null).await.unwrap();
    refused(t.k.run_action(&server, "RestaurantOrder", delivery, "dispatch", Value::Null).await, "invalid", "is not ready yet");
    let ready = t.k.run_action(&cook, "KitchenTicket", made, "ready", Value::Null).await.unwrap();
    assert!(ready["message"].as_str().unwrap().starts_with("Ready for Delivery"), "{ready}");
    let out = t.k.run_action(&server, "RestaurantOrder", delivery, "dispatch", Value::Null).await.unwrap();
    assert!(out["message"].as_str().unwrap().ends_with("is out for delivery"), "{out}");
    t.k.run_action(&server, "RestaurantOrder", delivery, "delivered", Value::Null).await.unwrap();
    assert_eq!(t.get("RestaurantOrder", delivery).await["delivery_status"], "Delivered");

    // ───────────── choices, seats, a moved table, payments at the table and stock ─────────────
    // What the kitchen uses comes from the outlet's store: a dish's recipe is its item's bill of
    // materials in ERP, and a bottled drink a stock item sold as it is.
    let products = t.find_one("ItemGroup", json!({"group_name": "Products"})).await;
    let kg = t.find_one("Uom", json!({"uom_name": "Kg"})).await;
    let stocked =
        |code: &str, uom: &Value| json!({"item_code": code, "item_name": code, "item_group": products["id"], "stock_uom": uom, "is_stock_item": true});
    let paneer = uid(&t.insert(&a, "Item", stocked("PANEER", &kg["id"])).await["id"]);
    let masala = uid(&t.insert(&a, "Item", stocked("MASALA", &kg["id"])).await["id"]);
    let cheese = uid(&t.insert(&a, "Item", stocked("CHEESE", &kg["id"])).await["id"]);
    let cola = uid(&t.insert(&a, "Item", stocked("COLA", &nos["id"])).await["id"]);
    let store = uid(&t.find_one("Warehouse", json!({"warehouse_name": "Stores", "company": company.to_string()})).await["id"]);
    let opening = uid(&t
        .insert(
            &a,
            "StockEntry",
            json!({"company": company.to_string(), "purpose": "Opening Stock", "items": [
                {"item": paneer.to_string(), "qty": "2", "basic_rate": "400", "target_warehouse": store.to_string()},
                {"item": masala.to_string(), "qty": "1", "basic_rate": "200", "target_warehouse": store.to_string()},
                {"item": cheese.to_string(), "qty": "1", "basic_rate": "600", "target_warehouse": store.to_string()},
                {"item": cola.to_string(), "qty": "10", "basic_rate": "30", "target_warehouse": store.to_string()}]}),
        )
        .await["id"]);
    t.k.submit(&a, "StockEntry", opening, None).await.expect("opening stock");
    let recipe = uid(&t
        .insert(
            &a,
            "Bom",
            json!({"company": company.to_string(), "item": tikka.to_string(), "quantity": "1", "items": [
                {"item": paneer.to_string(), "qty": "0.25"}, {"item": masala.to_string(), "qty": "0.02", "scrap_percent": "10"}]}),
        )
        .await["id"]);
    t.k.submit(&a, "Bom", recipe, None).await.expect("the tikka's recipe");
    let mut dishes = t.get("Menu", menu).await["dishes"].as_array().unwrap().clone();
    dishes.push(json!({"item": cola.to_string(), "dish_name": "Cola", "price": "40", "station": "Bar"}));
    t.k.update(
        &manager,
        "Menu",
        menu,
        obj(json!({"dishes": dishes, "modifiers": [
            {"modifier": "Extra Cheese", "dish": "Paneer Tikka", "price": "60", "item": cheese.to_string(), "qty": "0.05"},
            {"modifier": "No Onion"},
            {"modifier": "Truffle", "dish": "Paneer Tikka", "price": "200", "available": false}]})),
        None,
    )
    .await
    .expect("choices on the menu");
    // The outlet's store and accounts are its company's.
    let elsewhere = uid(&t.insert(&a, "Company", json!({"company_name": "Elsewhere", "abbr": "EW", "default_currency": inr["id"]})).await["id"]);
    let theirs = t.find_one("Warehouse", json!({"warehouse_name": "Stores", "company": elsewhere.to_string()})).await;
    refused(t.k.update(&manager, "Outlet", outlet, obj(json!({"store": theirs["id"]})), None).await, "invalid", "is another company's");
    let account = |name: &str| {
        let t = &t;
        let name = name.to_string();
        async move { uid(&t.find_one("Account", json!({"account_name": name, "company": company.to_string()})).await["id"]) }
    };
    let (cash, bank, liabilities) = (account("Cash").await, account("Primary Bank").await, account("Current Liabilities").await);
    t.k.update(&manager, "Outlet", outlet, obj(json!({"store": store.to_string(), "cash_account": cash.to_string(), "card_account": bank.to_string()})), None)
        .await
        .expect("the outlet's store and tills");
    let t3 = uid(&t.insert(&manager, "DiningTable", json!({"table_name": "3", "outlet": outlet.to_string(), "seats": 6})).await["id"]);

    // Choices are priced on top of their dish, as the menu writes them, and only those it offers.
    let dine = |table: Uuid, guests: i64, lines: Value| {
        obj(json!({"outlet": outlet.to_string(), "order_type": "Dine In", "table": table.to_string(), "guests": guests, "lines": lines}))
    };
    refused(
        t.k.insert(&server, "RestaurantOrder", dine(t2, 2, json!([{"dish": "Paneer Tikka", "qty": 1, "modifiers": "Truffle"}]))).await,
        "invalid",
        "Truffle is not available today",
    );
    refused(
        t.k.insert(&server, "RestaurantOrder", dine(t2, 2, json!([{"dish": "Paneer Tikka", "qty": 1, "modifiers": "Mint"}]))).await,
        "invalid",
        "Mint is not a choice for Paneer Tikka",
    );
    refused(
        t.k.insert(&server, "RestaurantOrder", dine(t2, 2, json!([{"dish": "Dal Makhani", "qty": 1, "modifiers": "extra cheese"}]))).await,
        "invalid",
        "extra cheese is not a choice for Dal Makhani",
    );
    let party =
        t.k.insert(
            &server,
            "RestaurantOrder",
            dine(
                t2,
                3,
                json!([{"dish": "Paneer Tikka", "qty": 2, "modifiers": "extra cheese, no onion", "seat": 1}, {"dish": "Cola", "qty": 1, "seat": 2},
                       {"dish": "Dal Makhani", "qty": 1}]),
            ),
        )
        .await
        .expect("an order with choices");
    assert_eq!(dec(&party["total"]), 1080.0, "2 × (350 + 60) + 40 + 220");
    assert_eq!(party["lines"][0]["modifiers"], "Extra Cheese, No Onion", "written as the menu writes them");
    let pid = uid(&party["id"]);
    // A new line's choices change: it is priced again.
    let mut ls = party["lines"].as_array().unwrap().clone();
    ls[0]["modifiers"] = json!("No Onion");
    let fewer = t.k.update(&server, "RestaurantOrder", pid, obj(json!({"lines": ls})), None).await.expect("no cheese");
    assert_eq!(dec(&fewer["total"]), 960.0, "2 × 350 + 40 + 220");
    let mut ls = fewer["lines"].as_array().unwrap().clone();
    ls[0]["modifiers"] = json!("No Onion, Extra Cheese");
    assert_eq!(dec(&t.k.update(&server, "RestaurantOrder", pid, obj(json!({"lines": ls})), None).await.expect("cheese after all")["total"]), 1080.0);
    // The grill is told the choices; sent, they are not changed.
    t.k.run_action(&server, "RestaurantOrder", pid, "send_to_kitchen", Value::Null).await.expect("sent");
    let grill = uid(&t.find_one("KitchenTicket", json!({"order": pid.to_string(), "station": "Grill"})).await["id"]);
    assert_eq!(t.get("KitchenTicket", grill).await["items"][0]["modifiers"], "No Onion, Extra Cheese");
    let mut ls = t.get("RestaurantOrder", pid).await["lines"].as_array().unwrap().clone();
    ls[0]["modifiers"] = json!("No Onion");
    refused(t.k.update(&server, "RestaurantOrder", pid, obj(json!({"lines": ls})), None).await, "invalid", "void it and order again");

    // The party moves to a bigger table; its tickets follow it.
    refused(t.k.run_action(&server, "RestaurantOrder", pid, "move_to", json!({"table": t2.to_string()})).await, "invalid", "is the order's table already");
    let moved = t.k.run_action(&server, "RestaurantOrder", pid, "move_to", json!({"table": t3.to_string()})).await.expect("moved");
    assert_eq!(moved["message"], "Moved to Table 3");
    assert!(!t.get("DiningTable", t2).await["occupied"].as_bool().unwrap() && t.get("DiningTable", t3).await["occupied"].as_bool().unwrap());
    assert_eq!(t.get("KitchenTicket", grill).await["table_label"], "Table 3");
    // Friends sat at the table it left join it: their order is merged into the party's.
    let late =
        uid(&t.k.insert(&server, "RestaurantOrder", dine(t2, 2, json!([{"dish": "Lime Soda", "qty": 2, "seat": 3}]))).await.expect("a second party")["id"]);
    t.k.run_action(&server, "RestaurantOrder", late, "send_to_kitchen", Value::Null).await.expect("to the bar");
    let merged = t.k.run_action(&server, "RestaurantOrder", late, "move_to", json!({"table": t3.to_string()})).await.expect("merged");
    let party_number = t.get("RestaurantOrder", pid).await["number"].as_str().unwrap().to_string();
    assert_eq!(merged["message"], format!("Merged into {party_number} at Table 3"));
    let gone = t.get("RestaurantOrder", late).await;
    assert_eq!((gone["status"].as_str(), gone["merged_into"].as_str(), dec(&gone["total"])), (Some("Merged"), Some(pid.to_string().as_str()), 0.0));
    let together = t.get("RestaurantOrder", pid).await;
    assert_eq!((dec(&together["total"]), together["guests"].as_i64(), together["lines"].as_array().unwrap().len()), (1260.0, Some(5), 4));
    assert!(!t.get("DiningTable", t2).await["occupied"].as_bool().unwrap(), "the table it left is free");
    // The bar's ticket for the sodas is the party's now, and the sodas follow it there.
    let mut sodas = None;
    for r in t.find("KitchenTicket", json!({"order": pid.to_string(), "station": "Bar"})).await {
        let ticket = t.get("KitchenTicket", uid(&r["id"])).await;
        if ticket["items"].as_array().unwrap().iter().any(|i| i["dish"] == "Lime Soda") {
            sodas = Some(uid(&ticket["id"]));
        }
    }
    let sodas = sodas.expect("the sodas' ticket went with them");
    t.k.run_action(&cook, "KitchenTicket", sodas, "start", Value::Null).await.unwrap();
    assert_eq!(t.k.run_action(&cook, "KitchenTicket", sodas, "ready", Value::Null).await.unwrap()["message"], "Ready for Table 3");
    assert!(lines(&t.get("RestaurantOrder", pid).await).iter().any(|(d, _, st)| d == "Lime Soda" && st == "Ready"), "the merged sodas follow their ticket");

    // A bill for each seat: every dish has its seat first.
    refused(t.k.run_action(&server, "RestaurantOrder", pid, "bill", json!({"split": "By Seat"})).await, "invalid", "Dal Makhani has no seat");
    let mut ls = t.get("RestaurantOrder", pid).await["lines"].as_array().unwrap().clone();
    for l in ls.iter_mut().filter(|l| l["dish"] == "Dal Makhani") {
        l["seat"] = json!(2);
    }
    t.k.update(&server, "RestaurantOrder", pid, obj(json!({"lines": ls})), None).await.expect("a seat for the dal, sent or not");
    let split = t.k.run_action(&server, "RestaurantOrder", pid, "bill", json!({"split": "By Seat"})).await.expect("billed by seat");
    assert!(split["message"].as_str().unwrap().starts_with("Billed by seat, 3 draft invoices in ERP: seat 1 "), "{split}");
    let billed = t.get("RestaurantOrder", pid).await;
    let bills = |o: &Doc| -> Vec<(i64, f64, f64, f64, String)> {
        o["bills"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| (b["seat"].as_i64().unwrap(), dec(&b["amount"]), dec(&b["paid"]), dec(&b["tips"]), b["status"].as_str().unwrap().to_string()))
            .collect()
    };
    assert_eq!(bills(&billed), vec![(1, 820.0, 0.0, 0.0, "Unpaid".into()), (2, 260.0, 0.0, 0.0, "Unpaid".into()), (3, 180.0, 0.0, 0.0, "Unpaid".into())]);
    assert!(billed["sales_invoice"].is_null(), "no one invoice: one for each seat");
    let seat_one = uid(&billed["bills"][0]["sales_invoice"]);
    assert_eq!(t.get("SalesInvoice", seat_one).await["items"][0]["description"], "Paneer Tikka (No Onion, Extra Cheese)");
    // What the dishes used left the store: the tikkas' recipe (its scrap too) and their extra
    // cheese, and the cola as it is; the dal and sodas use no stock. At the cost of goods sold.
    let used = t.get("StockEntry", uid(&billed["stock_entry"])).await;
    assert_eq!((used["purpose"].as_str(), used["docstatus"].as_i64()), (Some("Material Issue"), Some(1)));
    let mut took: Vec<(String, f64)> = used["items"].as_array().unwrap().iter().map(|i| (i["item"].as_str().unwrap().to_string(), dec(&i["qty"]))).collect();
    took.sort_by(|x, y| x.0.cmp(&y.0));
    let mut want = vec![(paneer.to_string(), 0.5), (masala.to_string(), 0.044), (cheese.to_string(), 0.1), (cola.to_string(), 1.0)];
    want.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(took, want);
    let cogs = t.get("Company", company).await["cogs_account"].clone();
    assert!(used["items"].as_array().unwrap().iter().all(|i| i["expense_account"] == cogs && i["source_warehouse"] == store.to_string().as_str()), "{used:?}");

    // Payment at the table, bill by bill.
    refused(t.k.run_action(&cook, "RestaurantOrder", pid, "settle", json!({"paid_by": "Card"})).await, "forbidden", "");
    refused(t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Card", "bill": 4})).await, "invalid", "is from 1 to 3");
    refused(t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Card", "tip": "80"})).await, "invalid", "no tips account");
    let tips = uid(&t
        .insert(
            &a,
            "Account",
            json!({"account_name": "Tips Payable", "company": company.to_string(), "parent_account": liabilities.to_string(), "root_type": "Liability"}),
        )
        .await["id"]);
    t.k.update(&manager, "Outlet", outlet, obj(json!({"tips_account": tips.to_string()})), None).await.expect("a tips account");
    let paid = t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Card", "tip": "80"})).await.expect("seat 1 pays by card");
    assert_eq!(paid["message"], "Took 820.00 by card and a tip of 80.00: bill 1 is paid");
    let pe = t.get("PaymentEntry", uid(&paid["payment_entry"]["id"])).await;
    assert_eq!(
        (dec(&pe["paid_amount"]), pe["docstatus"].as_i64(), pe["bank_account"].as_str(), dec(&pe["references"][0]["allocated_amount"])),
        (900.0, Some(1), Some(bank.to_string().as_str()), 820.0),
        "{pe:?}"
    );
    assert_eq!(
        (pe["deductions"][0]["account"].as_str(), dec(&pe["deductions"][0]["amount"])),
        (Some(tips.to_string().as_str()), -80.0),
        "the tip is credited to what the staff are owed"
    );
    let seat_one = t.get("SalesInvoice", seat_one).await;
    assert_eq!((seat_one["docstatus"].as_i64(), dec(&seat_one["outstanding_amount"])), (Some(1), 0.0), "submitted, and paid");
    // Seat 2 is shared between two guests paying cash.
    let half = t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Cash", "bill": 2, "amount": "130"})).await.expect("half");
    assert_eq!(half["message"], "Took 130.00 by cash: bill 2 owes 130.00");
    refused(
        t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Cash", "bill": 2, "amount": "200"})).await,
        "invalid",
        "bill 2 owes 130.00",
    );
    t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Cash", "bill": "2"})).await.expect("the other half");
    refused(t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Cash", "bill": 1})).await, "invalid", "bill 1 is paid");
    let last = t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Cash", "bill": 3})).await.expect("seat 3");
    assert_eq!(last["message"], "Took 180.00 by cash: bill 3 is paid; the order is settled");
    let settled = t.get("RestaurantOrder", pid).await;
    assert_eq!(settled["status"], "Settled");
    assert_eq!(bills(&settled), vec![(1, 820.0, 820.0, 80.0, "Paid".into()), (2, 260.0, 260.0, 0.0, "Paid".into()), (3, 180.0, 180.0, 0.0, "Paid".into())]);
    refused(t.k.run_action(&server, "RestaurantOrder", pid, "settle", json!({"paid_by": "Cash"})).await, "invalid", "not available");
    // The store runs short: nothing is billed until it holds what the dishes use.
    let thirsty = uid(&t.k.insert(&server, "RestaurantOrder", dine(t2, 2, json!([{"dish": "Cola", "qty": 20}]))).await.expect("a thirsty table")["id"]);
    refused(t.k.run_action(&server, "RestaurantOrder", thirsty, "bill", Value::Null).await, "invalid", "the store does not hold what its dishes use");
    assert_eq!(t.get("RestaurantOrder", thirsty).await["status"], "Open", "nothing billed");
    t.k.run_action(&server, "RestaurantOrder", thirsty, "cancel_order", Value::Null).await.expect("not sent: cancelled");

    // ───────────── a bill on a hotel guest's room ─────────────
    // Where the workspace runs a hotel too, a guest's bill may go on their room: Hospitality checks
    // the guest in it and posts it to their folio, and the hotel invoices it with the room.
    let night_item = uid(&t.insert(&a, "Item", item("ROOM-NIGHT")).await["id"]);
    let dining = uid(&t.insert(&a, "Item", item("DINING")).await["id"]);
    let inn = uid(&t
        .insert(
            &a,
            "HotelProperty",
            json!({"property_name": "The Tandoor Inn", "code": "INN", "company": company.to_string(), "room_item": night_item.to_string(),
                   "walk_in_customer": walk_in.to_string()}),
        )
        .await["id"]);
    let suite = uid(&t.insert(&a, "RoomType", json!({"room_type_name": "Suite", "property": inn.to_string(), "base_rate": "9000"})).await["id"]);
    let r7 = uid(&t.insert(&a, "Room", json!({"room_number": "7", "property": inn.to_string(), "room_type": suite.to_string()})).await["id"]);
    t.k.run_action(&a, "HotelProperty", inn, "open_business_date", json!({"date": "2026-12-01"})).await.expect("the inn opens");
    let ravi = uid(&t
        .insert(
            &a,
            "Stay",
            json!({"property": inn.to_string(), "guest_name": "Ravi Rao", "room_type": suite.to_string(), "arrival": "2026-12-01", "departure": "2026-12-03"}),
        )
        .await["id"]);
    t.k.run_action(&a, "Stay", ravi, "check_in", json!({"room": r7.to_string()})).await.expect("Ravi in room 7");
    let supper = uid(&t.k.insert(&server, "RestaurantOrder", dine(t2, 1, json!([{"dish": "Dal Makhani", "qty": 1}]))).await.expect("Ravi's supper")["id"]);
    t.k.run_action(&server, "RestaurantOrder", supper, "bill", Value::Null).await.expect("billed");
    let room_bill = |guest: &str, extra: Value| {
        let mut v = json!({"paid_by": "Room", "room": "7", "guest": guest});
        v.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap_or_default());
        v
    };
    refused(t.k.run_action(&server, "RestaurantOrder", supper, "settle", room_bill("Rao", json!({}))).await, "invalid", "names no hotel");
    t.k.update(&manager, "Outlet", outlet, obj(json!({"hotel_code": "INN", "room_charge_item": dining.to_string()})), None).await.expect("the outlet's hotel");
    // Putting a bill on a room is the hotel's Outlet Cashier's right.
    refused(t.k.run_action(&server, "RestaurantOrder", supper, "settle", room_bill("Rao", json!({}))).await, "forbidden", "");
    let host = t.user("host@tandoor.test", &["Server", "Outlet Cashier"]).await;
    let wrong_room = json!({"paid_by": "Room", "room": "9", "guest": "Rao"});
    refused(t.k.run_action(&host, "RestaurantOrder", supper, "settle", wrong_room).await, "invalid", "is not a room of hotel INN");
    refused(t.k.run_action(&host, "RestaurantOrder", supper, "settle", room_bill("Sen", json!({}))).await, "invalid", "is not the name of the guest in room 7");
    refused(t.k.run_action(&host, "RestaurantOrder", supper, "settle", room_bill("Rao", json!({"tip": "50"}))).await, "invalid", "is not put on a room");
    let billed_supper = t.get("RestaurantOrder", supper).await;
    let supper_invoice = uid(&billed_supper["bills"][0]["sales_invoice"]);
    let put = t.k.run_action(&host, "RestaurantOrder", supper, "settle", room_bill("rao", json!({}))).await.expect("on room 7");
    assert_eq!(put["message"], "Put on room 7: bill 1, 220.00; the order is settled");
    let after = t.get("RestaurantOrder", supper).await;
    assert_eq!((after["status"].as_str(), after["bills"][0]["status"].as_str()), (Some("Settled"), Some("On Room")));
    assert!(after["sales_invoice"].is_null() && after["bills"][0]["sales_invoice"].is_null(), "{after:?}");
    refused(t.k.get(&a, "SalesInvoice", supper_invoice, false).await, "not found", "");
    // A bill paid here in part does not go on a room.
    let snack = uid(&t.k.insert(&host, "RestaurantOrder", dine(t2, 1, json!([{"dish": "Lime Soda", "qty": 1}]))).await.expect("a soda")["id"]);
    t.k.run_action(&host, "RestaurantOrder", snack, "bill", Value::Null).await.expect("billed");
    t.k.run_action(&host, "RestaurantOrder", snack, "settle", json!({"paid_by": "Cash", "amount": "20"})).await.expect("part of it in cash");
    refused(t.k.run_action(&host, "RestaurantOrder", snack, "settle", room_bill("Rao", json!({}))).await, "invalid", "was paid here in part");
    t.k.run_action(&host, "RestaurantOrder", snack, "settle", json!({"paid_by": "Cash"})).await.expect("the rest in cash");
    let charge = t.get("RoomCharge", uid(&after["bills"][0]["room_charge"])).await;
    let folio = t.get("Folio", uid(&charge["folio"])).await;
    let on_folio = folio["charges"].as_array().unwrap().iter().find(|c| c["room_charge"] == charge["id"]).cloned().expect("on Ravi's folio");
    assert_eq!(
        (on_folio["description"].as_str().map(|d| d.starts_with("The Tandoor: Order ORD-")), dec(&on_folio["amount"]), on_folio["item"].as_str()),
        (Some(true), 220.0, Some(dining.to_string().as_str()))
    );

    // ───────────── reservations ─────────────
    let booking = |table: Uuid, party: i64, from: &str, to: &str| {
        obj(json!({"outlet": outlet.to_string(), "table": table.to_string(), "guest_name": "Cy", "party": party, "starts_at": from, "ends_at": to}))
    };
    refused(t.k.insert(&server, "TableReservation", booking(t2, 3, "2026-12-24T19:00:00Z", "2026-12-24T21:00:00Z")).await, "invalid", "the table seats 2");
    let held = uid(&t.k.insert(&server, "TableReservation", booking(t2, 2, "2026-12-24T19:00:00Z", "2026-12-24T21:00:00Z")).await.expect("booked")["id"]);
    refused(t.k.insert(&server, "TableReservation", booking(t2, 2, "2026-12-24T20:00:00Z", "2026-12-24T22:00:00Z")).await, "invalid", "held for Cy then");
    t.k.insert(&server, "TableReservation", booking(t2, 2, "2026-12-24T21:00:00Z", "2026-12-24T23:00:00Z")).await.expect("back to back");
    refused(t.k.insert(&server, "TableReservation", booking(t2, 2, "2026-12-24T19:00:00Z", "2026-12-24T18:00:00Z")).await, "invalid", "after it starts");
    t.k.update(&server, "TableReservation", held, obj(json!({"status": "Cancelled"})), None).await.unwrap();
    t.k.insert(&server, "TableReservation", booking(t2, 1, "2026-12-24T19:30:00Z", "2026-12-24T20:30:00Z")).await.expect("a cancelled booking frees the table");

    // ───────────── a guest's request ─────────────
    // Bo is known by a number only: the delivery and a table booked under it, written another way,
    // are hers; erasure blanks the name, number and address, and the order stays.
    let bo_booked = uid(&t
        .k
        .insert(
            &server,
            "TableReservation",
            obj(json!({"outlet": outlet.to_string(), "table": t2.to_string(), "guest_name": "Bo", "phone": "+91-98000-00000", "party": 2,
                       "starts_at": "2026-12-26T19:00:00Z", "ends_at": "2026-12-26T21:00:00Z"})),
        )
        .await
        .expect("booked")["id"]);
    let bo = Person { email: "bo@nowhere.test".into(), phone: Some("+919800000000".into()), name: None };
    let asked = t.k.personal_data_of(&t.admin, &bo).await.unwrap();
    let mut found: Vec<String> = asked["records"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap().to_string()).collect();
    found.sort();
    let mut hers = vec![delivery.to_string(), bo_booked.to_string()];
    hers.sort();
    assert_eq!(found, hers, "{asked}");
    let erased = t.k.erase_personal_data_of(&t.admin, &bo, "Request 1").await.unwrap();
    assert_eq!(erased["erased"].as_array().unwrap().len(), 2, "{erased}");
    let order = t.get("RestaurantOrder", delivery).await;
    assert!(order["phone"].is_null() && order["delivery_address"].is_null() && order["customer_name"] != "Bo", "{order:?}");
    assert_eq!(order["delivery_status"], "Delivered", "the order stays");
}
