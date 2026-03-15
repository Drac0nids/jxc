use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    models::{
        AuditLog, BarcodeLookupCache, Product, PurchaseOrder, SalesOrder, StockCheck, StockLog,
        User, UserRole, hash_password,
    },
    persistence::PersistenceHandles,
    repository::{RepositoryProvider, build_repository_provider},
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub persistence: PersistenceHandles,
    pub repository: RepositoryProvider,
    pub users: Arc<Mutex<HashMap<String, User>>>,
    pub products: Arc<Mutex<HashMap<i64, Product>>>,
    pub next_product_id: Arc<Mutex<i64>>,
    pub barcode_index: Arc<Mutex<HashMap<String, i64>>>,
    pub barcode_lookup_cache: Arc<Mutex<HashMap<String, BarcodeLookupCache>>>,
    pub purchase_orders: Arc<Mutex<HashMap<i64, PurchaseOrder>>>,
    pub next_purchase_order_id: Arc<Mutex<i64>>,
    pub sales_orders: Arc<Mutex<HashMap<i64, SalesOrder>>>,
    pub next_sales_order_id: Arc<Mutex<i64>>,
    pub stock_checks: Arc<Mutex<HashMap<i64, StockCheck>>>,
    pub next_stock_check_id: Arc<Mutex<i64>>,
    pub stock_logs: Arc<Mutex<Vec<StockLog>>>,
    pub audit_logs: Arc<Mutex<Vec<AuditLog>>>,
    pub idempotency_records: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let repository = build_repository_provider(config.storage_backend);
        let persistence = PersistenceHandles::memory();
        let tenant_id = Uuid::new_v4();
        let owner = User {
            id: Uuid::new_v4(),
            tenant_id,
            username: "admin".to_string(),
            name: "系统管理员".to_string(),
            role: UserRole::Owner,
            password_hash: hash_password("admin123"),
        };

        let sample_product = Product {
            id: 1001,
            tenant_id,
            sku: "KO-330".to_string(),
            barcode: "690123456789".to_string(),
            name: "可口可乐 330ml".to_string(),
            unit: "罐".to_string(),
            current_stock: 100,
            cost_price: Decimal::new(210, 2),
            retail_price: Decimal::new(350, 2),
            last_inbound_unit_cost: Some(Decimal::new(320, 2)),
            min_stock_limit: 10,
            is_deleted: false,
            category_id: None,
            track_batches: false,
        };

        let mut users = HashMap::new();
        users.insert(owner.username.clone(), owner);

        let mut products = HashMap::new();
        products.insert(sample_product.id, sample_product.clone());

        let mut barcode_index = HashMap::new();
        barcode_index.insert(
            format!("{}:{}", tenant_id, sample_product.barcode.clone()),
            sample_product.id,
        );

        Self {
            config,
            persistence,
            repository,
            users: Arc::new(Mutex::new(users)),
            products: Arc::new(Mutex::new(products)),
            next_product_id: Arc::new(Mutex::new(2000)),
            barcode_index: Arc::new(Mutex::new(barcode_index)),
            barcode_lookup_cache: Arc::new(Mutex::new(HashMap::new())),
            purchase_orders: Arc::new(Mutex::new(HashMap::new())),
            next_purchase_order_id: Arc::new(Mutex::new(3000)),
            sales_orders: Arc::new(Mutex::new(HashMap::new())),
            next_sales_order_id: Arc::new(Mutex::new(4000)),
            stock_checks: Arc::new(Mutex::new(HashMap::new())),
            next_stock_check_id: Arc::new(Mutex::new(5000)),
            stock_logs: Arc::new(Mutex::new(Vec::new())),
            audit_logs: Arc::new(Mutex::new(Vec::new())),
            idempotency_records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_runtime_infra(
        mut self,
        persistence: PersistenceHandles,
        repository: RepositoryProvider,
    ) -> Self {
        self.persistence = persistence;
        self.repository = repository;
        self
    }
}
