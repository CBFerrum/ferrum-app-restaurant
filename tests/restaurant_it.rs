//! An evening at a restaurant (FERRUM-INDUSTRY-002) against a real PostgreSQL
//! (`FERRUM_RESTAURANT_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_restaurant_test`;
//! reset first): a table's order priced from the menu, one open order to a table, its lines sent to
//! the kitchen as a ticket for each station and worked through to served, sent lines voided rather
//! than edited, the bill made through ERP and the table freed; a takeaway and a delivery; table
//! reservations that hold one party at a time within the seats; and who may do what.

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
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    ferrum_app_erp::register(&mut hooks);
    ferrum_app_restaurant::register(&mut hooks);
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
            packages: vec!["restaurant".into()],
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
    let oid = uid(&first["id"]);
    assert!(t.get("DiningTable", t1).await["occupied"].as_bool().unwrap(), "the table is taken");
    refused(t.k.insert(&server, "RestaurantOrder", order(Some(t1), "Dine In", &[("Lime Soda", 1)], json!({}))).await, "invalid", "has an open order");
    refused(t.k.insert(&cook, "RestaurantOrder", order(Some(t2), "Dine In", &[("Lime Soda", 1)], json!({}))).await, "forbidden", "");

    // To the kitchen: a ticket for each station.
    let sent = t.k.run_action(&server, "RestaurantOrder", oid, "send_to_kitchen", Value::Null).await.expect("sent");
    assert_eq!(sent["tickets"].as_array().unwrap().len(), 3);
    let tickets = t.find("KitchenTicket", json!({"order": oid.to_string()})).await;
    let mut stations: Vec<String> = tickets.iter().map(|x| x["station"].as_str().unwrap().to_string()).collect();
    stations.sort();
    assert_eq!(stations, ["Bar", "Grill", "Kitchen"]);
    refused(t.k.run_action(&server, "RestaurantOrder", oid, "send_to_kitchen", Value::Null).await, "invalid", "nothing new");
    let now = t.get("RestaurantOrder", oid).await;
    assert!(lines(&now).iter().all(|(_, _, st)| st == "Sent"));
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
    t.k.run_action(&server, "RestaurantOrder", delivery, "dispatch", Value::Null).await.unwrap();
    t.k.run_action(&server, "RestaurantOrder", delivery, "delivered", Value::Null).await.unwrap();
    assert_eq!(t.get("RestaurantOrder", delivery).await["delivery_status"], "Delivered");

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
