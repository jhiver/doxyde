use criterion::{black_box, criterion_group, criterion_main, Criterion};
use doxyde_core::models::{Page, Site};
use doxyde_db::repositories::{PageRepository, SiteRepository};
use sqlx::SqlitePool;

fn create_test_pool() -> SqlitePool {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    })
}

fn bench_site_operations(c: &mut Criterion) {
    let pool = create_test_pool();
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create test site
    let site_repo = SiteRepository::new(pool.clone());
    rt.block_on(async {
        let site = Site::new(
            "bench-domain.test".to_string(),
            "Benchmark Site".to_string(),
        );
        site_repo.create(&site).await.unwrap()
    });

    // Benchmark site retrieval by domain
    c.bench_function("site_find_by_domain", |b| {
        b.iter(|| {
            rt.block_on(async {
                site_repo
                    .find_by_domain(black_box("bench-domain.test"))
                    .await
                    .expect("Failed to find site")
            })
        });
    });
}

fn bench_page_operations(c: &mut Criterion) {
    let pool = create_test_pool();
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create test site and pages
    let site_repo = SiteRepository::new(pool.clone());
    let page_repo = PageRepository::new(pool.clone());

    let site_id = rt.block_on(async {
        let site = Site::new("bench.test".to_string(), "Benchmark Site".to_string());
        site_repo
            .create(&site)
            .await
            .expect("Failed to create site")
    });

    // Create test pages
    let page_ids: Vec<i64> = rt.block_on(async {
        let mut ids = Vec::new();
        for i in 0..10 {
            let page = Page::new(site_id, format!("page-{}", i), format!("Page {}", i));
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

    // Benchmark listing pages by site
    c.bench_function("page_list_by_site_id", |b| {
        b.iter(|| {
            rt.block_on(async {
                page_repo
                    .list_by_site_id(black_box(site_id))
                    .await
                    .expect("Failed to list pages")
            })
        });
    });
}

criterion_group!(benches, bench_site_operations, bench_page_operations);
criterion_main!(benches);
