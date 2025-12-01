use super::common;

use common::test_server::TestServer;
use metis::adapters::api_handler::{ApiResponse, ResourceDto};
use serde_json::json;

#[tokio::test]
async fn test_resource_tags_persistence() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    // 1. Create a resource with initial tags
    let resource_dto = ResourceDto {
        uri: "test://resource".to_string(),
        name: "Test Resource".to_string(),
        description: Some("A test resource".to_string()),
        mime_type: Some("text/plain".to_string()),
        tags: vec!["initial".to_string(), "tag".to_string()],
        output_schema: None,
        content: Some("content".to_string()),
        mock: None,
    };

    let response = client
        .post(server.url("/api/resources"))
        .json(&resource_dto)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);

    // 2. Verify initial tags
    let response = client
        .get(server.url("/api/resources/test%3A%2F%2Fresource"))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let body: ApiResponse<ResourceDto> = response.json().await.unwrap();
    let fetched_resource = body.data.unwrap();
    assert_eq!(fetched_resource.tags, vec!["initial", "tag"]);

    // 3. Update the resource with NEW tags
    let mut updated_dto = resource_dto.clone();
    updated_dto.tags = vec!["updated".to_string(), "tags".to_string(), "working".to_string()];

    let response = client
        .put(server.url("/api/resources/test%3A%2F%2Fresource"))
        .json(&updated_dto)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // 4. Verify UPDATED tags
    let response = client
        .get(server.url("/api/resources/test%3A%2F%2Fresource"))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let body: ApiResponse<ResourceDto> = response.json().await.unwrap();
    let fetched_resource = body.data.unwrap();
    
    // This assertion is expected to FAIL before the fix
    assert_eq!(fetched_resource.tags, vec!["updated", "tags", "working"], "Tags should be updated after PUT request");
}
