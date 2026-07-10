// Booking JSON variants integration tests
// Uses axum_test::TestServer + wiremock to verify JSON contract on booking endpoints.

use axum::http::StatusCode;
use axum_test::TestServer;
use doxyde_web::routes::create_router;
use serde_json::json;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

async fn setup_test() -> (TestServer, sqlx::SqlitePool, MockServer, tempfile::TempDir) {
    let mock_server = MockServer::start().await;

    // Create temp directory for sites
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_string_lossy().to_string();

    // Create test app state
    let mut state = doxyde_web::test_helpers::create_test_app_state()
        .await
        .expect("Failed to create test app state");

    // Override sites directory
    state.config.sites_directory = temp_path.clone();

    // Get DB pool for test.local
    let base_path = std::path::PathBuf::from(&temp_path);
    let context = doxyde_web::site_resolver::SiteContext::new("test.local".to_string(), &base_path);
    let pool = state
        .db_router
        .get_pool(&context)
        .await
        .expect("Failed to get pool");

    // Configure booking config with mock server
    sqlx::query("UPDATE booking_config SET service_url = ?, service_secret = ? WHERE id = 1")
        .bind(mock_server.uri())
        .bind("test-secret")
        .execute(&pool)
        .await
        .expect("Failed to update booking config");

    // Add booking listings
    sqlx::query(
        "INSERT INTO booking_listing (listing_id, role, position, page_path) VALUES (?, ?, ?, ?)",
    )
    .bind(123)
    .bind("primary")
    .bind(0)
    .bind("/stars")
    .execute(&pool)
    .await
    .expect("Failed to insert booking listing");

    // Setup dummy templates for booking
    let mut tera = tera::Tera::default();
    tera.add_raw_template("page_move.html", "Move page")
        .unwrap();
    tera.add_raw_template("page_delete.html", "Delete page")
        .unwrap();
    tera.add_raw_template("login.html", "Login").unwrap();
    tera.add_raw_template("booking/stay.html", "Stay Search HTML")
        .unwrap();
    tera.add_raw_template("booking/book.html", "Book stay HTML")
        .unwrap();

    state.templates = doxyde_web::autoreload_templates::TemplateEngine::Static {
        templates_dir: "templates".to_string(),
        tera: std::sync::Arc::new(tera),
    };

    // Create router and TestServer
    let app = create_router(state);
    let server = TestServer::new(app).expect("Failed to create test server");

    (server, pool, mock_server, temp_dir)
}

#[tokio::test]
async fn test_stay_json_valid_dates() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let mock_response = json!({
        "check_in": "2099-01-01",
        "check_out": "2099-01-05",
        "nights": 4,
        "adults": 2,
        "children": 0,
        "infants": 0,
        "results": [
            {
                "name": "Cozy Stay",
                "is_multi_stay": false,
                "leg_count": 1,
                "check_in": "2099-01-01",
                "check_out": "2099-01-05",
                "nights": 4,
                "person_capacity": 4,
                "currency_code": "EUR",
                "total_price": 400.0,
                "price_is_estimate": false,
                "images": [],
                "legs": [
                    {
                        "listing_id": 123,
                        "name": "Cozy Stay Leg",
                        "check_in": "2099-01-01",
                        "check_out": "2099-01-05",
                        "nights": 4,
                        "price": 400.0,
                        "currency_code": "EUR",
                        "images": []
                    }
                ]
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1/availability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let response = server
        .get("/.stay?from=2099-01-01&to=2099-01-05&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);

    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "ok");
    assert_eq!(json_body["nights"], 4);
    assert_eq!(json_body["currency_code"], "EUR");

    let results = json_body["results"]
        .as_array()
        .expect("results is not an array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["name"], "Cozy Stay");
    assert_eq!(results[0]["page_url"], "/stars");
}

#[tokio::test]
async fn test_stay_json_not_configured() {
    let (server, pool, _mock_server, _temp_dir) = setup_test().await;

    // Clear service_url to make it not configured
    sqlx::query("UPDATE booking_config SET service_url = '' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    let response = server
        .get("/.stay?from=2099-01-01&to=2099-01-05&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "error");
    assert_eq!(json_body["code"], "not_configured");
}

#[tokio::test]
async fn test_stay_json_invalid_dates() {
    let (server, _pool, _mock_server, _temp_dir) = setup_test().await;

    let response = server
        .get("/.stay?from=2099-01-05&to=2099-01-01&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "error");
    assert_eq!(json_body["code"], "invalid_dates");
}

#[tokio::test]
async fn test_stay_json_past_date() {
    let (server, _pool, _mock_server, _temp_dir) = setup_test().await;

    let response = server
        .get("/.stay?from=2020-01-01&to=2020-01-05&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "error");
    assert_eq!(json_body["code"], "past_date");
}

#[tokio::test]
async fn test_book_json_quote() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let mock_quote = json!({
        "listing_id": 123,
        "name": "Cozy Stay",
        "person_capacity": 4,
        "images": [],
        "check_in": "2099-01-01",
        "check_out": "2099-01-05",
        "nights": 4,
        "available": true,
        "currency_code": "EUR",
        "total_price": 400.0,
        "components": null
    });

    let mock_calendar = json!({
        "listing_id": 123,
        "min_date": "2099-01-01",
        "max_date": "2099-12-31",
        "blocked": ["2099-01-10"]
    });

    Mock::given(method("POST"))
        .and(path("/v1/quote"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_quote))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/calendar/123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_calendar))
        .mount(&mock_server)
        .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "ok");
    assert_eq!(json_body["quote"]["listing_id"], 123);
    assert_eq!(json_body["quote"]["total_price"], 400.0);
    assert_eq!(json_body["blocked_dates"], json!(["2099-01-10"]));
}

#[tokio::test]
async fn test_book_json_create_confirmed() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let mock_reservation = json!({
        "reservation_id": 456,
        "confirmation_code": "CONF123",
        "listing_id": 123,
        "check_in": "2099-01-01",
        "check_out": "2099-01-05",
        "nights": 4,
        "total_price": 400.0,
        "currency_code": "EUR",
        "status": "confirmed",
        "payment_status": "unpaid"
    });

    Mock::given(method("POST"))
        .and(path("/v1/reservations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_reservation))
        .mount(&mock_server)
        .await;

    let form_data = [
        ("listing_id", "123"),
        ("from", "2099-01-01"),
        ("to", "2099-01-05"),
        ("first_name", "John"),
        ("last_name", "Doe"),
        ("email", "john.doe@example.com"),
        ("format", "json"),
    ];

    let response = server
        .post("/.book")
        .add_header("Host", "test.local")
        .form(&form_data)
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "confirmed");
    assert_eq!(json_body["reservation"]["reservation_id"], 456);
}

#[tokio::test]
async fn test_book_json_create_error() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    Mock::given(method("POST"))
        .and(path("/v1/reservations"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let form_data = [
        ("listing_id", "123"),
        ("from", "2099-01-01"),
        ("to", "2099-01-05"),
        ("first_name", "John"),
        ("last_name", "Doe"),
        ("email", "john.doe@example.com"),
        ("format", "json"),
    ];

    let response = server
        .post("/.book")
        .add_header("Host", "test.local")
        .form(&form_data)
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "error");
    assert_eq!(json_body["code"], "booking_error");
}

#[tokio::test]
async fn test_stay_html_default() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let mock_response = json!({
        "check_in": "2099-01-01",
        "check_out": "2099-01-05",
        "nights": 4,
        "adults": 2,
        "children": 0,
        "infants": 0,
        "results": []
    });

    Mock::given(method("POST"))
        .and(path("/v1/availability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let response = server
        .get("/.stay?from=2099-01-01&to=2099-01-05")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    response.assert_text("Stay Search HTML");
}

#[tokio::test]
async fn test_stay_json_accept_header() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let mock_response = json!({
        "check_in": "2099-01-01",
        "check_out": "2099-01-05",
        "nights": 4,
        "adults": 2,
        "children": 0,
        "infants": 0,
        "results": []
    });

    Mock::given(method("POST"))
        .and(path("/v1/availability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    // Case 1: Accept: application/json header, no format parameter -> JSON response
    let response = server
        .get("/.stay?from=2099-01-01&to=2099-01-05")
        .add_header("Host", "test.local")
        .add_header("Accept", "application/json")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "ok");

    // Case 2: format=json parameter + Accept: text/html header -> JSON response (param wins)
    let response = server
        .get("/.stay?from=2099-01-01&to=2099-01-05&format=json")
        .add_header("Host", "test.local")
        .add_header("Accept", "text/html")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "ok");
}

#[tokio::test]
async fn test_book_html_create_confirmed() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let mock_reservation = json!({
        "reservation_id": 456,
        "confirmation_code": "CONF123",
        "listing_id": 123,
        "check_in": "2099-01-01",
        "check_out": "2099-01-05",
        "nights": 4,
        "total_price": 400.0,
        "currency_code": "EUR",
        "status": "confirmed",
        "payment_status": "unpaid"
    });

    Mock::given(method("POST"))
        .and(path("/v1/reservations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_reservation))
        .mount(&mock_server)
        .await;

    let form_data = [
        ("listing_id", "123"),
        ("from", "2099-01-01"),
        ("to", "2099-01-05"),
        ("first_name", "John"),
        ("last_name", "Doe"),
        ("email", "john.doe@example.com"),
    ];

    let response = server
        .post("/.book")
        .add_header("Host", "test.local")
        .form(&form_data)
        .await;

    response.assert_status(StatusCode::OK);
    response.assert_header("content-type", "text/html; charset=utf-8");
    response.assert_text("Book stay HTML");
}

#[tokio::test]
async fn test_book_json_not_configured() {
    let (server, pool, _mock_server, _temp_dir) = setup_test().await;

    // Clear service_url to make it not configured
    sqlx::query("UPDATE booking_config SET service_url = '' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "error");
    assert_eq!(json_body["code"], "not_configured");
}
