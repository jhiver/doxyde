use criterion::{black_box, criterion_group, criterion_main, Criterion};
use doxyde_core::models::Page;
use doxyde_db::repositories::PageRepository;
use sqlx::SqlitePool;

fn create_test_pool() -> SqlitePool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // Initialize site_config for single-database architecture
        sqlx::query("INSERT OR IGNORE INTO site_config (id, title) VALUES (1, 'Benchmark Site')")
            .execute(&pool)
            .await
            .expect("Failed to create site_config");

        pool
    })
}

fn bench_page_operations(c: &mut Criterion) {
    let pool = create_test_pool();
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create test pages (no site_id needed in single-database architecture)
    let page_repo = PageRepository::new(pool.clone());

    // Create test pages
    let page_ids: Vec<i64> = rt.block_on(async {
        let mut ids = Vec::new();
        for i in 0..10 {
            let page = Page::new(format!("page-{}", i), format!("Page {}", i));
            ids.push(page_repo.create(&page).await.unwrap());
        }
        ids
    });

    // Benchmark page retrieval by ID
    c.bench_function("page_find_by_id", |b| {
        b.iter(|| {
            rt.block_on(async {
                page_repo
                    .find_by_id(black_box(page_ids[0]))
                    .await
                    .expect("Failed to find page")
            })
        });
    });

    // Benchmark listing all pages
    c.bench_function("page_list_all", |b| {
        b.iter(|| {
            rt.block_on(async {
                page_repo
                    .list_all()
                    .await
                    .expect("Failed to list pages")
            })
        });
    });
}

criterion_group!(benches, bench_page_operations);
criterion_main!(benches);
