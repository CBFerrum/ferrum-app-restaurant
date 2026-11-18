//! Double-booking under load against a real PostgreSQL (`FERRUM_RESTAURANT_TEST_DATABASE_URL`,
//! default `postgres://ferrum:ferrum@localhost:5432/ferrum_restaurant_test`; reset first): fifty
//! requests for one table at the same moment, each in its own transaction on its own connection,
//! and the table goes to exactly one of them, for a reservation's time and for an open order.

use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// How many ask at once.
const AT_ONCE: usize = 50;

/// One database: the tests here take turns, each resetting it.
static TURN: Mutex<()> = Mutex::new(());

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn run(scenario: impl Future<Output = ()> + Send + 'static) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario).await.unwrap() });
}

struct Floor {
    k: Arc<Kernel>,
    admin: Ctx,
    outlet: String,
    table: String,
}

impl Floor {
    /// A restaurant with one table for four.
    async fn new() -> Floor {
        let url =
            std::env::var("FERRUM_RESTAURANT_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_restaurant_test".into());
        // A connection for each request, and one for each side transaction a number takes.
        let db = Db::connect(DbConfig { url, max_connections: 2 * AT_ONCE as u32 + 8, ..Default::default() }).await.expect("connect");
        sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
        let mut packages = ferrum_domain::packages().expect("platform packages");
        packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
        packages.extend(ferrum_app_restaurant::packages().expect("app packages"));
        let mut hooks = HookRegistry::default();
        ferrum_domain::register(&mut hooks);
        hooks.app("erp", &[], ferrum_app_erp::register);
        ferrum_app_restaurant::register(&mut hooks);
        let k = Kernel::boot(db, Catalog::from_packages(packages).expect("catalog"), hooks, KernelSettings::default(), None).await.expect("boot");
        ferrum_domain::maintenance(&k).await.expect("partitions");
        let tenant = k
            .create_tenant(NewTenant {
                slug: "rush".into(),
                name: "The Rush".into(),
                admin_email: "admin@rush.test".into(),
                admin_password: "Correct-Horse-9".into(),
                admin_name: None,
                packages: vec!["restaurant".into()],
                settings: Some(json!({"fiscal_year_start_month": 4, "timezone": "Asia/Kolkata"})),
            })
            .await
            .expect("tenant");
        let session = k.login("rush", "admin@rush.test", "Correct-Horse-9").await.expect("login");
        let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
        let mut f = Floor { k, admin, outlet: String::new(), table: String::new() };
        let inr = f.find("Currency", json!({"code": "INR"})).await.remove(0)["id"].clone();
        let company = f.insert("Company", json!({"company_name": "The Rush", "abbr": "TR", "default_currency": inr})).await;
        let walk_in = f.insert("Customer", json!({"customer_name": "Walk-in", "customer_type": "Individual"})).await;
        f.outlet = f.insert("Outlet", json!({"outlet_name": "The Rush", "company": company, "walk_in_customer": walk_in})).await;
        f.table = f.insert("DiningTable", json!({"table_name": "7", "outlet": f.outlet, "seats": 4})).await;
        f
    }

    async fn insert(&self, entity: &str, v: Value) -> String {
        let doc = self.k.insert(&self.admin, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"));
        doc["id"].as_str().unwrap().to_string()
    }

    async fn find(&self, entity: &str, filters: Value) -> Vec<Doc> {
        let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), limit: Some(500), ..Default::default() };
        self.k.list(&self.admin, entity, &q).await.unwrap().rows
    }

    /// `AT_ONCE` inserts of `entity`, released together, each in its own transaction: how many were
    /// saved, and the refusals.
    async fn rush(&self, entity: &'static str, doc: Value) -> (usize, Vec<Error>) {
        // Each one's connection open before they start, so none waits for another to be made.
        let open: Vec<_> = (0..AT_ONCE).map(|_| tokio::spawn(self.k.db.pool.acquire())).collect();
        for c in open {
            drop(c.await.unwrap().expect("a connection"));
        }
        let gate = Arc::new(tokio::sync::Barrier::new(AT_ONCE));
        let tasks: Vec<_> = (0..AT_ONCE)
            .map(|_| {
                let (k, admin, gate, doc) = (self.k.clone(), self.admin.clone(), gate.clone(), obj(doc.clone()));
                tokio::spawn(async move {
                    gate.wait().await;
                    k.insert(&admin, entity, doc).await
                })
            })
            .collect();
        let (mut saved, mut refused) = (0, Vec::new());
        for t in tasks {
            match t.await.unwrap() {
                Ok(_) => saved += 1,
                Err(e) => refused.push(e),
            }
        }
        (saved, refused)
    }
}

#[test]
fn fifty_reservations_of_one_table_at_one_time_hold_it_once() {
    run(async {
        let f = Floor::new().await;
        let booking = json!({"outlet": f.outlet, "table": f.table, "guest_name": "Cy", "party": 2,
                             "starts_at": "2026-12-24T19:00:00Z", "ends_at": "2026-12-24T21:00:00Z"});
        let (saved, refused) = f.rush("TableReservation", booking).await;
        assert_eq!(saved, 1, "{AT_ONCE} reservations of one table at one time: {saved} were saved");
        assert!(refused.iter().all(|e| format!("{e:?}").contains("held for")), "refused because the table is held: {refused:?}");
        assert_eq!(f.find("TableReservation", json!({"table": f.table, "status": "Booked"})).await.len(), 1);
    });
}

#[test]
fn fifty_orders_on_one_free_table_open_one() {
    run(async {
        let f = Floor::new().await;
        let services = f.find("ItemGroup", json!({"group_name": "Services"})).await.remove(0)["id"].clone();
        let nos = f.find("Uom", json!({"uom_name": "Nos"})).await.remove(0)["id"].clone();
        let dal = f.insert("Item", json!({"item_code": "DAL", "item_name": "DAL", "item_group": services, "stock_uom": nos, "is_stock_item": false})).await;
        let menu = f
            .insert(
                "Menu",
                json!({"menu_name": "Dinner", "outlet": f.outlet, "dishes": [{"item": dal, "dish_name": "Dal Makhani", "price": "220", "station": "Kitchen"}]}),
            )
            .await;
        f.k.update(&f.admin, "Outlet", f.outlet.parse().unwrap(), obj(json!({"menu": menu})), None).await.expect("the outlet's menu");
        let order = json!({"outlet": f.outlet, "order_type": "Dine In", "table": f.table, "guests": 2, "lines": [{"dish": "Dal Makhani", "qty": 1}]});
        let (saved, refused) = f.rush("RestaurantOrder", order).await;
        assert_eq!(saved, 1, "{AT_ONCE} orders on one free table at one time: {saved} were opened");
        assert!(refused.iter().all(|e| format!("{e:?}").contains("has an open order")), "refused because the table has its order: {refused:?}");
        assert_eq!(f.find("RestaurantOrder", json!({"table": f.table, "status": "Open"})).await.len(), 1);
    });
}
