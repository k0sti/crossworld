//! XCube API client implementation

use crate::types::{XCubeError, XCubeModel, XCubeResponse};
use reqwest::Client;

/// Base URL for XCube API
const XCUBE_API_BASE: &str = "https://api.xcube.example.com";

/// XCube API client for fetching voxel models
pub struct XCubeClient {
    client: Client,
    base_url: String,
}

impl XCubeClient {
    /// Create a new XCube client with default base URL
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: XCUBE_API_BASE.to_string(),
        }
    }

    /// Create a new XCube client with custom base URL
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Fetch a model by ID from XCube API
    pub async fn fetch_model(&self, model_id: &str) -> Result<XCubeModel, XCubeError> {
        let url = format!("{}/models/{}", self.base_url, model_id);

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(XCubeError::ModelNotFound(model_id.to_string()));
        }

        let xcube_response: XCubeResponse = response.json().await?;

        if !xcube_response.success {
            return Err(XCubeError::InvalidModelData(
                xcube_response.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        xcube_response.data.ok_or_else(|| {
            XCubeError::InvalidModelData("No model data in response".to_string())
        })
    }

    /// Search for models by name or author
    pub async fn search_models(&self, query: &str) -> Result<Vec<XCubeModel>, XCubeError> {
        let url = format!("{}/models/search?q={}", self.base_url, query);

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(XCubeError::RequestFailed(
                response.error_for_status().unwrap_err()
            ));
        }

        let models: Vec<XCubeModel> = response.json().await?;
        Ok(models)
    }

    /// List popular or featured models
    pub async fn list_featured(&self, limit: Option<u32>) -> Result<Vec<XCubeModel>, XCubeError> {
        let mut url = format!("{}/models/featured", self.base_url);
        if let Some(limit) = limit {
            url.push_str(&format!("?limit={}", limit));
        }

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(XCubeError::RequestFailed(
                response.error_for_status().unwrap_err()
            ));
        }

        let models: Vec<XCubeModel> = response.json().await?;
        Ok(models)
    }
}

impl Default for XCubeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = XCubeClient::new();
        assert_eq!(client.base_url, XCUBE_API_BASE);
    }

    #[test]
    fn test_client_with_custom_url() {
        let custom_url = "https://custom.xcube.com".to_string();
        let client = XCubeClient::with_base_url(custom_url.clone());
        assert_eq!(client.base_url, custom_url);
    }
}
