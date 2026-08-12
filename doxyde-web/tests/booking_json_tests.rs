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

fn select_fragment<'a>(html: &'a str, id: &str) -> &'a str {
    let marker = format!("<select id=\"{id}\"");
    let start = html.find(&marker).expect("select not rendered");
    let end = html[start..]
        .find("</select>")
        .expect("select is not closed");
    &html[start..start + end]
}

async fn mount_quote(mock_server: &MockServer, quote: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/v1/quote"))
        .respond_with(ResponseTemplate::new(200).set_body_json(quote))
        .mount(mock_server)
        .await;
}

fn valid_quote() -> serde_json::Value {
    json!({
        "listing_id": 123,
        "name": "Cozy Stay",
        "person_capacity": 4,
        "children_allowed": true,
        "infants_allowed": true,
        "max_children_allowed": null,
        "max_infants_allowed": null,
        "images": [],
        "check_in": "2099-01-01",
        "check_out": "2099-01-05",
        "nights": 4,
        "available": true,
        "currency_code": "EUR",
        "total_price": 400.0,
        "components": null
    })
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
        "children_allowed": true,
        "infants_allowed": true,
        "max_children_allowed": null,
        "max_infants_allowed": null,
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
async fn test_book_json_rejects_old_quote_response_missing_policy_fields() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    mount_quote(
        &mock_server,
        json!({
            "listing_id": 123,
            "name": "Old API Stay",
            "person_capacity": 4,
            "images": [],
            "check_in": "2099-01-01",
            "check_out": "2099-01-05",
            "nights": 4,
            "available": true
        }),
    )
    .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "error");
    assert_eq!(json_body["code"], "service_error");

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/v1/quote");
}

#[tokio::test]
async fn test_book_json_create_confirmed() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    mount_quote(&mock_server, valid_quote()).await;

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
        ("adults", "2"),
        ("children", "1"),
        ("infants", "0"),
        ("first_name", "John"),
        ("last_name", "Doe"),
        ("email", "john.doe@example.com"),
        ("utm_source", "newsletter"),
        ("gclid", "gclid-123"),
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

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url.path(), "/v1/quote");
    assert_eq!(requests[1].url.path(), "/v1/reservations");
    let quote_body: serde_json::Value = requests[0].body_json().unwrap();
    assert_eq!(quote_body["listing_id"], 123);
    assert_eq!(quote_body["from"], "2099-01-01");
    assert_eq!(quote_body["to"], "2099-01-05");
    assert_eq!(quote_body["adults"], 2);
    assert_eq!(quote_body["children"], 1);
    assert_eq!(quote_body["infants"], 0);
    let reservation_body: serde_json::Value = requests[1].body_json().unwrap();
    assert_eq!(reservation_body["adults"], 2);
    assert_eq!(reservation_body["children"], 1);
    assert_eq!(reservation_body["infants"], 0);
    assert_eq!(reservation_body["attribution"]["utm_source"], "newsletter");
    assert_eq!(reservation_body["attribution"]["gclid"], "gclid-123");
}

#[tokio::test]
async fn test_book_json_rejects_old_quote_before_reservation() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    mount_quote(
        &mock_server,
        json!({
            "listing_id": 123,
            "check_in": "2099-01-01",
            "check_out": "2099-01-05",
            "nights": 4,
            "available": true
        }),
    )
    .await;

    let form_data = [
        ("listing_id", "123"),
        ("from", "2099-01-01"),
        ("to", "2099-01-05"),
        ("adults", "2"),
        ("children", "0"),
        ("infants", "0"),
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

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/v1/quote");
}

#[tokio::test]
async fn test_book_json_create_error() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    mount_quote(&mock_server, valid_quote()).await;

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

    mount_quote(&mock_server, valid_quote()).await;

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
async fn test_book_json_not_configured_precedes_listing_selection_for_get_and_post() {
    let (server, pool, mock_server, _temp_dir) = setup_test().await;

    // Clear both booking prerequisites to verify configuration takes precedence.
    sqlx::query("UPDATE booking_config SET service_url = '' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM booking_listing")
        .execute(&pool)
        .await
        .unwrap();

    let get_response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&format=json")
        .add_header("Host", "test.local")
        .await;

    get_response.assert_status(StatusCode::OK);
    let get_json = get_response.json::<serde_json::Value>();
    assert_eq!(get_json["status"], "error");
    assert_eq!(get_json["code"], "not_configured");

    let form_data = [
        ("listing_id", "123"),
        ("from", "2099-01-01"),
        ("to", "2099-01-05"),
        ("first_name", "John"),
        ("last_name", "Doe"),
        ("email", "john.doe@example.com"),
        ("format", "json"),
    ];
    let post_response = server
        .post("/.book")
        .add_header("Host", "test.local")
        .form(&form_data)
        .await;

    post_response.assert_status(StatusCode::OK);
    let post_json = post_response.json::<serde_json::Value>();
    assert_eq!(post_json["status"], "error");
    assert_eq!(post_json["code"], "not_configured");
    assert!(mock_server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_book_get_rejects_unselected_listing_before_sejours_call() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let response = server
        .get("/.book?listing=999&from=2099-01-01&to=2099-01-05&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "error");
    assert_eq!(json_body["code"], "listing_not_selected");
    assert!(mock_server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_book_post_rejects_unselected_listing_before_sejours_call() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let form_data = [
        ("listing_id", "999"),
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
    assert_eq!(json_body["code"], "listing_not_selected");
    assert!(mock_server.received_requests().await.unwrap().is_empty());
}

async fn setup_test_with_real_templates(
) -> (TestServer, sqlx::SqlitePool, MockServer, tempfile::TempDir) {
    let mock_server = MockServer::start().await;
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_string_lossy().to_string();

    let mut state = doxyde_web::test_helpers::create_test_app_state()
        .await
        .expect("Failed to create test app state");
    state.config.sites_directory = temp_path.clone();

    let base_path = std::path::PathBuf::from(&temp_path);
    let context = doxyde_web::site_resolver::SiteContext::new("test.local".to_string(), &base_path);
    let pool = state
        .db_router
        .get_pool(&context)
        .await
        .expect("Failed to get pool");

    sqlx::query("UPDATE booking_config SET service_url = ?, service_secret = ? WHERE id = 1")
        .bind(mock_server.uri())
        .bind("test-secret")
        .execute(&pool)
        .await
        .expect("Failed to update booking config");

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

    state.templates = doxyde_web::templates::init_templates("../templates", false)
        .expect("Failed to initialize templates");

    let app = create_router(state);
    let server = TestServer::new(app).expect("Failed to create test server");

    (server, pool, mock_server, temp_dir)
}

#[tokio::test]
async fn test_book_html_rejects_unselected_listing_without_service_details() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    let response = server
        .get("/.book?listing=999&from=2099-01-01&to=2099-01-05")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(html.contains("This stay is not available for booking on this site."));
    assert!(!html.contains("test-secret"));
    assert!(!html.contains("id=\"bk-adults\""));
    assert!(!html.contains("id=\"bk-checkin\""));
    assert!(!html.contains("var guestPolicy"));
    assert!(mock_server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_book_html_quote_failure_renders_no_booking_controls() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    Mock::given(method("POST"))
        .and(path("/v1/quote"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(html.contains("The booking service is temporarily unavailable."));
    assert!(!html.contains("id=\"bk-adults\""));
    assert!(!html.contains("id=\"bk-checkin\""));
    assert!(!html.contains("<div class=\"booking-price-card\">"));
    assert!(!html.contains("var guestPolicy"));
    assert!(!html.contains("flatpickr.min.js"));
}

#[tokio::test]
async fn test_book_html_capacity_two_bounds_guest_controls_and_hidden_values() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;
    mount_quote(
        &mock_server,
        json!({
            "listing_id": 123,
            "name": "Small Stay",
            "person_capacity": 2,
            "children_allowed": true,
            "infants_allowed": true,
            "max_children_allowed": null,
            "max_infants_allowed": null,
            "images": [],
            "check_in": "2099-01-01",
            "check_out": "2099-01-05",
            "nights": 4,
            "available": true,
            "currency_code": "EUR",
            "total_price": 200.0,
            "components": null
        }),
    )
    .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&adults=1&children=1&infants=1&utm_source=google&gclid=abc%20123")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    let adults = select_fragment(&html, "bk-adults");
    let children = select_fragment(&html, "bk-children");
    assert!(adults.contains("value=\"1\""));
    assert!(adults.contains("value=\"2\""));
    assert!(!adults.contains("value=\"3\""));
    assert!(children.contains("value=\"0\""));
    assert!(children.contains("value=\"1\""));
    assert!(!children.contains("value=\"2\""));
    assert!(html.contains("name=\"adults\" value=\"1\""));
    assert!(html.contains("name=\"children\" value=\"1\""));
    assert!(html.contains("name=\"infants\" value=\"1\""));
    assert!(html.contains("&utm_source=google&gclid=abc%20123"));
    assert!(html.contains("class=\"booking-contact-form\""));
    assert!(html.contains("var guestPolicy = {"));
}

#[tokio::test]
async fn test_book_html_reservation_failure_renders_policy_aware_controls() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;
    mount_quote(
        &mock_server,
        json!({
            "listing_id": 123,
            "name": "Policy Stay",
            "person_capacity": 3,
            "children_allowed": true,
            "infants_allowed": true,
            "max_children_allowed": 1,
            "max_infants_allowed": null,
            "images": [],
            "check_in": "2099-01-01",
            "check_out": "2099-01-05",
            "nights": 4,
            "available": true,
            "currency_code": "EUR",
            "total_price": 300.0,
            "components": null
        }),
    )
    .await;

    Mock::given(method("POST"))
        .and(path("/v1/reservations"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let form_data = [
        ("listing_id", "123"),
        ("from", "2099-01-01"),
        ("to", "2099-01-05"),
        ("adults", "2"),
        ("children", "1"),
        ("infants", "0"),
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
    let html = response.text();
    assert!(html.contains("We could not complete your booking."));
    assert!(html.contains("id=\"bk-checkin\""));
    assert!(html.contains("class=\"booking-contact-form\""));
    assert!(html.contains("\"person_capacity\":3"));

    let adults = select_fragment(&html, "bk-adults");
    assert!(adults.contains("value=\"1\""));
    assert!(adults.contains("value=\"2\""));
    assert!(adults.contains("value=\"3\""));
    assert!(!adults.contains("value=\"4\""));

    let children = select_fragment(&html, "bk-children");
    assert!(children.contains("value=\"0\""));
    assert!(children.contains("value=\"1\""));
    assert!(!children.contains("value=\"2\""));
}

#[tokio::test]
async fn test_book_html_unavailable_quote_keeps_policy_controls_without_contact_form() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;
    let mut unavailable_quote = valid_quote();
    unavailable_quote["available"] = json!(false);
    mount_quote(&mock_server, unavailable_quote).await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&adults=2&children=1&infants=1")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(html.contains("These dates are no longer available."));
    assert!(html.contains("id=\"bk-checkin\""));
    assert!(html.contains("id=\"bk-adults\""));
    assert!(html.contains("id=\"bk-children\""));
    assert!(html.contains("var guestPolicy = {"));
    assert!(html.contains("var i = \"1\";"));
    assert!(!html.contains("name=\"infants\""));
    assert!(!html.contains("class=\"booking-contact-form\""));
    assert!(!html.contains("id=\"first_name\""));
}

#[tokio::test]
async fn test_book_html_adult_options_follow_capacity_without_default_cap() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;
    let mut quote = valid_quote();
    quote["person_capacity"] = json!(8);
    mount_quote(&mock_server, quote).await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&adults=7&children=0&infants=0")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    let adults = select_fragment(&html, "bk-adults");
    assert!(adults.contains("value=\"7\" selected"));
    assert!(adults.contains("value=\"8\""));
    assert!(!adults.contains("value=\"9\""));
    assert!(html.contains("name=\"adults\" value=\"7\""));
}

#[tokio::test]
async fn test_book_html_children_forbidden_renders_adults_only() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;
    mount_quote(
        &mock_server,
        json!({
            "listing_id": 123,
            "name": "Adults Stay",
            "person_capacity": 2,
            "children_allowed": false,
            "infants_allowed": false,
            "max_children_allowed": null,
            "max_infants_allowed": null,
            "images": [],
            "check_in": "2099-01-01",
            "check_out": "2099-01-05",
            "nights": 4,
            "available": true,
            "currency_code": "EUR",
            "total_price": 200.0,
            "components": null
        }),
    )
    .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&adults=2&children=0&infants=0")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(html.contains("Adults only"));
    assert!(!html.contains("id=\"bk-children\""));
    assert!(html.contains("name=\"children\" value=\"0\""));
}

#[tokio::test]
async fn test_book_html_children_are_limited_by_remaining_capacity() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;
    mount_quote(
        &mock_server,
        json!({
            "listing_id": 123,
            "name": "Three Person Stay",
            "person_capacity": 3,
            "children_allowed": true,
            "infants_allowed": true,
            "max_children_allowed": 4,
            "max_infants_allowed": null,
            "images": [],
            "check_in": "2099-01-01",
            "check_out": "2099-01-05",
            "nights": 4,
            "available": true,
            "currency_code": "EUR",
            "total_price": 300.0,
            "components": null
        }),
    )
    .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&adults=2&children=1")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    let children = select_fragment(&html, "bk-children");
    assert!(children.contains("value=\"1\""));
    assert!(!children.contains("value=\"2\""));
}

#[tokio::test]
async fn test_book_html_children_above_default_limit_remain_selected_and_synced() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;
    mount_quote(
        &mock_server,
        json!({
            "listing_id": 123,
            "name": "Large Stay",
            "person_capacity": 8,
            "children_allowed": true,
            "infants_allowed": true,
            "max_children_allowed": null,
            "max_infants_allowed": null,
            "images": [],
            "check_in": "2099-01-01",
            "check_out": "2099-01-05",
            "nights": 4,
            "available": true,
            "currency_code": "EUR",
            "total_price": 800.0,
            "components": null
        }),
    )
    .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&adults=2&children=5")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    let children = select_fragment(&html, "bk-children");
    assert!(children.contains("value=\"5\" selected"));
    assert!(children.contains("value=\"6\""));
    assert!(html.contains("name=\"children\" value=\"5\""));
    assert!(html.contains("Math.max(0, guestPolicy.max_children_allowed)"));
    assert!(html.contains("var current = parseInt(childrenEl.value, 10);"));
    assert!(html.contains("var selected = Math.min(Math.max(current, 0), maximum);"));
    assert!(!html.contains("Math.min(4, maximum)"));
}

#[tokio::test]
async fn test_book_html_children_respect_explicit_maximum() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;
    mount_quote(
        &mock_server,
        json!({
            "listing_id": 123,
            "name": "Child Limited Stay",
            "person_capacity": 6,
            "children_allowed": true,
            "infants_allowed": true,
            "max_children_allowed": 1,
            "max_infants_allowed": null,
            "images": [],
            "check_in": "2099-01-01",
            "check_out": "2099-01-05",
            "nights": 4,
            "available": true,
            "currency_code": "EUR",
            "total_price": 300.0,
            "components": null
        }),
    )
    .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&adults=2&children=1")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    let children = select_fragment(&html, "bk-children");
    assert!(children.contains("value=\"1\""));
    assert!(!children.contains("value=\"2\""));
    assert!(html.contains("\"max_children_allowed\":1"));
}

#[tokio::test]
async fn test_book_json_maps_structured_guest_policy_error() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;
    Mock::given(method("POST"))
        .and(path("/v1/quote"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "detail": {"code": "children_not_allowed", "message": "policy"}
        })))
        .mount(&mock_server)
        .await;

    let response = server
        .get("/.book?listing=123&from=2099-01-01&to=2099-01-05&adults=1&children=1&format=json")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let json_body = response.json::<serde_json::Value>();
    assert_eq!(json_body["status"], "error");
    assert_eq!(json_body["code"], "children_not_allowed");
}

#[tokio::test]
async fn test_book_json_maps_guest_policy_unavailable_without_reservation() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;
    Mock::given(method("POST"))
        .and(path("/v1/quote"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "detail": {"code": "guest_policy_unavailable", "message": "policy"}
        })))
        .mount(&mock_server)
        .await;

    let form_data = [
        ("listing_id", "123"),
        ("from", "2099-01-01"),
        ("to", "2099-01-05"),
        ("adults", "1"),
        ("children", "1"),
        ("infants", "0"),
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
    assert_eq!(json_body["code"], "guest_policy_unavailable");

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/v1/quote");
}

#[tokio::test]
async fn test_book_json_maps_reservation_guest_policy_error() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;
    mount_quote(&mock_server, valid_quote()).await;

    Mock::given(method("POST"))
        .and(path("/v1/reservations"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "detail": {"code": "infants_not_allowed", "message": "policy"}
        })))
        .mount(&mock_server)
        .await;

    let form_data = [
        ("listing_id", "123"),
        ("from", "2099-01-01"),
        ("to", "2099-01-05"),
        ("adults", "1"),
        ("children", "0"),
        ("infants", "1"),
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
    assert_eq!(json_body["code"], "infants_not_allowed");
}

#[tokio::test]
async fn test_suggested_stays_no_date_uses_only_primary_site_listings() {
    let (server, pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    sqlx::query(
        "INSERT INTO booking_listing (listing_id, role, position, page_path) VALUES (?, ?, ?, ?)",
    )
    .bind(456)
    .bind("secondary")
    .bind(1)
    .bind("/villa")
    .execute(&pool)
    .await
    .unwrap();

    let mock_response = json!({
        "suggestions": [
            {
                "listing_id": 123,
                "listing_name": "Sunset Villa",
                "check_in": "2099-08-01",
                "check_out": "2099-08-04",
                "nights": 3,
                "price_total": 350.0,
                "currency_code": "EUR",
                "image": "https://example.com/sunset.jpg",
                "person_capacity": 4
            }
        ],
        "degraded": false
    });

    Mock::given(method("POST"))
        .and(path("/v1/suggested-stays"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let response = server.get("/.stay").add_header("Host", "test.local").await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(html.contains("Suggested stays"));
    assert!(html.contains("Sunset Villa"));
    assert!(html.contains("2099-08-01 → 2099-08-04"));
    assert!(html.contains("350 EUR"));
    assert!(
        html.contains("https:&#x2F;&#x2F;example.com&#x2F;sunset.jpg"),
        "rendered HTML did not contain the suggestion image: {html}"
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    let listing_ids = body["listing_ids"].as_array().unwrap();
    let ids: Vec<i64> = listing_ids.iter().map(|v| v.as_i64().unwrap()).collect();
    assert_eq!(ids, vec![123]);
}

#[tokio::test]
async fn test_suggested_stays_link_params_and_encoded_attribution() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    let mock_response = json!({
        "suggestions": [
            {
                "listing_id": 123,
                "listing_name": "Ocean Loft",
                "check_in": "2099-09-10",
                "check_out": "2099-09-13",
                "nights": 3,
                "price_total": 200.0,
                "currency_code": "EUR",
                "image": null,
                "person_capacity": 2
            }
        ],
        "degraded": false
    });

    Mock::given(method("POST"))
        .and(path("/v1/suggested-stays"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let response = server
        .get("/.stay?adults=3&children=1&infants=1&utm_source=google&gclid=abc%20123")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(
        html.contains("/.stay?from=2099-09-10&to=2099-09-13&adults=3&children=1&infants=1&utm_source=google&gclid=abc%20123"),
        "rendered HTML did not contain the attributed suggestion link: {html}"
    );
}

#[tokio::test]
async fn test_suggested_stays_degraded_uses_next_availability_and_no_scarcity() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    let mock_response = json!({
        "suggestions": [
            {
                "listing_id": 123,
                "listing_name": "Beach House",
                "check_in": "2099-10-01",
                "check_out": "2099-10-05",
                "nights": 4,
                "price_total": 500.0,
                "currency_code": "EUR",
                "image": null,
                "person_capacity": 4
            }
        ],
        "degraded": true
    });

    Mock::given(method("POST"))
        .and(path("/v1/suggested-stays"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let response = server.get("/.stay").add_header("Host", "test.local").await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(html.contains("Next availability"));
    assert!(!html.contains("Suggested stays"));

    let lower_html = html.to_lowercase();
    assert!(!lower_html.contains("last room"));
    assert!(!lower_html.contains("last night"));
    assert!(!lower_html.contains("hurry"));
    assert!(!lower_html.contains("only 1 left"));
}

#[tokio::test]
async fn test_suggested_stays_empty_suggestions_renders_no_module() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    let mock_response = json!({
        "suggestions": [],
        "degraded": false
    });

    Mock::given(method("POST"))
        .and(path("/v1/suggested-stays"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let response = server.get("/.stay").add_header("Host", "test.local").await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(!html.contains("Suggested stays"));
    assert!(!html.contains("Next availability"));
}

#[tokio::test]
async fn test_suggested_stays_api_failure_silently_degrades() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    Mock::given(method("POST"))
        .and(path("/v1/suggested-stays"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let response = server.get("/.stay").add_header("Host", "test.local").await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(!html.contains("The booking service is temporarily unavailable"));
    assert!(!html.contains("Suggested stays"));
}

#[tokio::test]
async fn test_dated_request_calls_availability_and_not_suggested_stays() {
    let (server, _pool, mock_server, _temp_dir) = setup_test().await;

    let mock_avail = json!({
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
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_avail))
        .mount(&mock_server)
        .await;

    let response = server
        .get("/.stay?from=2099-01-01&to=2099-01-05")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/v1/availability");
}

#[tokio::test]
async fn test_partial_dates_preserve_normal_form_without_suggestions_call() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    let response = server
        .get("/.stay?from=2099-01-01")
        .add_header("Host", "test.local")
        .await;

    response.assert_status(StatusCode::OK);
    let html = response.text();
    assert!(!html.contains("Suggested stays"));
    assert!(!html.contains("Next availability"));

    let requests = mock_server.received_requests().await.unwrap();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn test_default_visible_dates_are_tomorrow_and_day_after() {
    let (server, _pool, mock_server, _temp_dir) = setup_test_with_real_templates().await;

    let mock_response = json!({
        "suggestions": [],
        "degraded": false
    });

    Mock::given(method("POST"))
        .and(path("/v1/suggested-stays"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let response = server.get("/.stay").add_header("Host", "test.local").await;

    response.assert_status(StatusCode::OK);
    let html = response.text();

    let today = chrono::Utc::now()
        .with_timezone(&chrono_tz::Indian::Mauritius)
        .date_naive();
    let expected_from = (today + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let expected_to = (today + chrono::Duration::days(2))
        .format("%Y-%m-%d")
        .to_string();

    assert!(html.contains(&format!("value=\"{}\"", expected_from)));
    assert!(html.contains(&format!("value=\"{}\"", expected_to)));
}

#[test]
fn test_suggested_stay_deserialization_nullable_fields() {
    use doxyde_web::services::sejours_client::SuggestedStaysResponse;

    let json_data = r#"{
        "suggestions": [
            {
                "listing_id": 999,
                "listing_name": "Unit 999",
                "check_in": "2026-08-01",
                "check_out": "2026-08-02",
                "nights": 1,
                "price_total": null,
                "currency_code": null,
                "image": null,
                "person_capacity": null
            }
        ],
        "generated_at": "2026-07-23T12:00:00Z",
        "degraded": true
    }"#;

    let res: SuggestedStaysResponse =
        serde_json::from_str(json_data).expect("deserialization failed");
    assert!(res.degraded);
    assert_eq!(res.suggestions.len(), 1);
    let sug = &res.suggestions[0];
    assert_eq!(sug.listing_id, 999);
    assert_eq!(sug.check_in, "2026-08-01");
    assert_eq!(sug.check_out, "2026-08-02");
    assert_eq!(sug.nights, 1);
    assert!(sug.price_total.is_none());
    assert!(sug.currency_code.is_none());
    assert!(sug.image.is_none());
    assert!(sug.person_capacity.is_none());
    assert_eq!(sug.listing_name, "Unit 999");
}

#[test]
fn test_booking_policy_fields_are_required_from_upgraded_api() {
    use doxyde_web::services::sejours_client::{Listing, QuoteResponse};

    assert!(serde_json::from_str::<Listing>(r#"{"listing_id": 1}"#).is_err());
    assert!(serde_json::from_str::<QuoteResponse>(
        r#"{
            "listing_id": 1,
            "check_in": "2099-01-01",
            "check_out": "2099-01-02",
            "nights": 1,
            "available": true
        }"#,
    )
    .is_err());
    assert!(serde_json::from_str::<QuoteResponse>(
        r#"{
            "listing_id": 1,
            "children_allowed": true,
            "infants_allowed": true,
            "max_children_allowed": null,
            "check_in": "2099-01-01",
            "check_out": "2099-01-02",
            "nights": 1,
            "available": true
        }"#,
    )
    .is_err());
}
