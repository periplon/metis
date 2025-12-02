use super::common;

use common::test_server::TestServer;

#[tokio::test]
async fn test_health_endpoint() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
    assert!(body["uptime_seconds"].is_number());
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_health_ready_endpoint() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/health/ready"))
        .send()
        .await
        .unwrap();

    // May be 200 or 503 depending on config
    assert!(response.status() == 200 || response.status() == 503);
}

#[tokio::test]
async fn test_health_live_endpoint() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/health/live"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "alive");
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let response = client
        .get(server.url("/metrics"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body = response.text().await.unwrap();
    // Should contain Prometheus metrics
    assert!(body.contains("metis_"));
}
