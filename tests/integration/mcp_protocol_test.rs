mod common;

use common::test_server::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_mcp_initialize() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client
        .post(server.url("/mcp"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert!(body["result"]["protocolVersion"].is_string());
    assert!(body["result"]["capabilities"].is_object());
}

#[tokio::test]
async fn test_mcp_ping() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping"
    });

    let response = client
        .post(server.url("/mcp"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 2);
    assert!(body["result"].is_object());
}

#[tokio::test]
async fn test_mcp_resources_list() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/list"
    });

    let response = client
        .post(server.url("/mcp"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 3);
    assert!(body["result"]["resources"].is_array());
}

#[tokio::test]
async fn test_mcp_tools_list() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/list"
    });

    let response = client
        .post(server.url("/mcp"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 4);
    assert!(body["result"]["tools"].is_array());
}

#[tokio::test]
async fn test_mcp_prompts_list() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "prompts/list"
    });

    let response = client
        .post(server.url("/mcp"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 5);
    assert!(body["result"]["prompts"].is_array());
}

#[tokio::test]
async fn test_mcp_method_not_found() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "unknown/method"
    });

    let response = client
        .post(server.url("/mcp"))
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 6);
    assert!(body["error"].is_object());
    assert_eq!(body["error"]["code"], -32601);
}
