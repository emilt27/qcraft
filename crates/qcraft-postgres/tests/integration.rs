//! Single integration test binary for all PostgreSQL integration tests.
//! Uses ONE shared container for all tests, with template databases for isolation.

use std::sync::LazyLock;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use postgres::{Client, NoTls};
use testcontainers::ImageExt;
use testcontainers::runners::SyncRunner;
use testcontainers_modules::postgres::Postgres;

mod common;

mod integration {
    pub mod ddl;
    pub mod ddl_ignored_features;
    pub mod dml;
    pub mod dml_ignored_features;
    pub mod dml_unsupported_features;
    pub mod dql;
    pub mod dql_ignored_features;
    pub mod dql_unsupported_features;
    pub mod tcl;
    pub mod tcl_ignored_features;
    pub mod tcl_unsupported_features;
}

// Global container ID for atexit cleanup
static CONTAINER_ID: OnceLock<String> = OnceLock::new();

extern "C" fn stop_container() {
    if let Some(id) = CONTAINER_ID.get() {
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", id])
            .output();
    }
}

struct TestDb {
    host: String,
    port: u16,
    _container: Box<dyn std::any::Any + Send + Sync>,
}

static TEST_DB: LazyLock<TestDb> = LazyLock::new(|| {
    let node = Postgres::default()
        .with_tag("16-alpine")
        .with_cmd(["postgres", "-c", "max_prepared_transactions=10"])
        .start()
        .unwrap();
    let host = node.get_host().unwrap().to_string();
    let port = node.get_host_port_ipv4(5432).unwrap();

    // Store container ID for atexit cleanup
    CONTAINER_ID.set(node.id().to_string()).ok();
    unsafe {
        libc::atexit(stop_container);
    }

    let conn_str =
        format!("host={host} port={port} user=postgres password=postgres dbname=postgres");
    let mut client = Client::connect(&conn_str, NoTls).unwrap();

    // Create all template databases (each must be a separate statement —
    // CREATE DATABASE cannot run inside a transaction block)
    client
        .batch_execute("CREATE DATABASE template_dql TEMPLATE template0")
        .unwrap();
    client
        .batch_execute("CREATE DATABASE template_dql_ign TEMPLATE template0")
        .unwrap();
    client
        .batch_execute("CREATE DATABASE template_tcl TEMPLATE template0")
        .unwrap();
    client
        .batch_execute("CREATE DATABASE template_dml_ign TEMPLATE template0")
        .unwrap();
    drop(client);

    // Seed template_dql: users, orders, products with data
    {
        let conn =
            format!("host={host} port={port} user=postgres password=postgres dbname=template_dql");
        let mut c = Client::connect(&conn, NoTls).unwrap();
        c.batch_execute(
            "
            CREATE TABLE \"users\" (
                \"id\" INTEGER PRIMARY KEY,
                \"name\" TEXT NOT NULL,
                \"email\" TEXT UNIQUE,
                \"age\" INTEGER,
                \"active\" BOOLEAN NOT NULL DEFAULT TRUE,
                \"department\" TEXT
            );
            CREATE TABLE \"orders\" (
                \"id\" INTEGER PRIMARY KEY,
                \"user_id\" INTEGER NOT NULL REFERENCES \"users\"(\"id\"),
                \"product\" TEXT NOT NULL,
                \"amount\" NUMERIC(10,2) NOT NULL,
                \"created_at\" DATE NOT NULL
            );
            CREATE TABLE \"products\" (
                \"id\" INTEGER PRIMARY KEY,
                \"name\" TEXT NOT NULL,
                \"price\" NUMERIC(10,2) NOT NULL,
                \"category\" TEXT NOT NULL
            );

            INSERT INTO \"users\" VALUES (1, 'Alice', 'alice@example.com', 30, TRUE, 'engineering');
            INSERT INTO \"users\" VALUES (2, 'Bob', 'bob@example.com', 25, TRUE, 'engineering');
            INSERT INTO \"users\" VALUES (3, 'Charlie', 'charlie@example.com', 35, FALSE, 'sales');
            INSERT INTO \"users\" VALUES (4, 'Diana', 'diana@example.com', 28, TRUE, 'sales');
            INSERT INTO \"users\" VALUES (5, 'Eve', 'eve@example.com', NULL, TRUE, 'engineering');

            INSERT INTO \"orders\" VALUES (1, 1, 'Widget', 10.50, '2024-01-15');
            INSERT INTO \"orders\" VALUES (2, 1, 'Gadget', 25.00, '2024-01-20');
            INSERT INTO \"orders\" VALUES (3, 2, 'Widget', 10.50, '2024-02-01');
            INSERT INTO \"orders\" VALUES (4, 4, 'Gizmo', 50.00, '2024-02-15');
            INSERT INTO \"orders\" VALUES (5, 4, 'Widget', 10.50, '2024-03-01');

            INSERT INTO \"products\" VALUES (1, 'Widget', 10.50, 'hardware');
            INSERT INTO \"products\" VALUES (2, 'Gadget', 25.00, 'electronics');
            INSERT INTO \"products\" VALUES (3, 'Gizmo', 50.00, 'electronics');
            INSERT INTO \"products\" VALUES (4, 'Doohickey', 5.00, 'hardware');
            ",
        )
        .unwrap();
    }

    // Seed template_dql_ign: simple users table
    {
        let conn = format!(
            "host={host} port={port} user=postgres password=postgres dbname=template_dql_ign"
        );
        let mut c = Client::connect(&conn, NoTls).unwrap();
        c.batch_execute(
            "
            CREATE TABLE \"users\" (
                \"id\" INTEGER PRIMARY KEY,
                \"name\" TEXT NOT NULL,
                \"active\" BOOLEAN NOT NULL DEFAULT TRUE
            );
            INSERT INTO \"users\" VALUES (1, 'Alice', TRUE);
            INSERT INTO \"users\" VALUES (2, 'Bob', TRUE);
            INSERT INTO \"users\" VALUES (3, 'Charlie', FALSE);
            ",
        )
        .unwrap();
    }

    // Seed template_tcl: table t for transaction tests
    {
        let conn =
            format!("host={host} port={port} user=postgres password=postgres dbname=template_tcl");
        let mut c = Client::connect(&conn, NoTls).unwrap();
        c.batch_execute("CREATE TABLE \"t\" (\"id\" INTEGER)")
            .unwrap();
    }

    // Seed template_dml_ign: table t for DML ignored features tests
    {
        let conn = format!(
            "host={host} port={port} user=postgres password=postgres dbname=template_dml_ign"
        );
        let mut c = Client::connect(&conn, NoTls).unwrap();
        c.batch_execute("CREATE TABLE \"t\" (\"id\" SERIAL PRIMARY KEY, \"val\" TEXT)")
            .unwrap();
    }

    TestDb {
        host,
        port,
        _container: Box::new(node),
    }
});

static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Create a test database cloned from a template, return a connected client.
pub(crate) fn test_client(template: &str) -> Client {
    let db = &*TEST_DB;
    let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let test_db = format!("test_{n}");

    let admin_conn = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        db.host, db.port
    );
    let mut admin = Client::connect(&admin_conn, NoTls).unwrap();
    admin
        .batch_execute(&format!(
            "CREATE DATABASE \"{test_db}\" TEMPLATE \"{template}\""
        ))
        .unwrap();
    drop(admin);

    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname={test_db}",
        db.host, db.port
    );
    Client::connect(&conn_str, NoTls).unwrap()
}
