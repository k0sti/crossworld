//! HTTP client for Roblox Cube3D inference server
//!
//! Provides async HTTP communication with the Cube3D Python server,
//! including automatic retry logic and error handling.

use crate::types::{
    GenerationRequest, OccupancyRequest, OccupancyResult, Result, RobocubeError, RobocubeResult,
    ServerStatus,
};
use reqwest::Client;
use std::time::Duration;

/// Default server URL
pub const DEFAULT_SERVER_URL: &str = "http://localhost:8642";

/// HTTP client for Cube3D text-to-3D generation
///
/// # Example
///
/// ```no_run
/// use robocube::{RobocubeClient, GenerationRequest};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = RobocubeClient::new("http://localhost:8642");
///
///     // Check server health
///     let status = client.health_check().await?;
///     println!("Server: {}", status.status);
///
///     // Generate from prompt
///     let request = GenerationRequest::new("A wooden chair");
///     let result = client.generate(&request).await?;
///
///     println!("Generated {} vertices", result.vertex_count());
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RobocubeClient {
    client: Client,
    base_url: String,
    health_timeout: Duration,
    generate_timeout: Duration,
    max_retries: u32,
    base_delay_ms: u64,
}

impl RobocubeClient {
    /// Create a new client with default settings
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of the Cube3D server (e.g., "http://localhost:8642")
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut url = base_url.into();
        // Ensure no trailing slash for consistent URL building
        if url.ends_with('/') {
            url.pop();
        }

        Self {
            client: Client::new(),
            base_url: url,
            health_timeout: Duration::from_secs(5),
            generate_timeout: Duration::from_secs(600), // 10 minutes for generation
            max_retries: 3,
            base_delay_ms: 1000,
        }
    }

    /// Create a new client from environment variable or default
    pub fn from_env() -> Self {
        let url =
            std::env::var("ROBOCUBE_SERVER_URL").unwrap_or_else(|_| DEFAULT_SERVER_URL.to_string());
        Self::new(url)
    }

    /// Set custom health check timeout
    pub fn with_health_timeout(mut self, timeout: Duration) -> Self {
        self.health_timeout = timeout;
        self
    }

    /// Set custom generation timeout
    pub fn with_generate_timeout(mut self, timeout: Duration) -> Self {
        self.generate_timeout = timeout;
        self
    }

    /// Set maximum retry attempts
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set base delay for exponential backoff (in milliseconds)
    pub fn with_base_delay_ms(mut self, delay_ms: u64) -> Self {
        self.base_delay_ms = delay_ms;
        self
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Check server health status
    ///
    /// Returns server status including GPU availability, model loading state,
    /// and any error messages.
    pub async fn health_check(&self) -> Result<ServerStatus> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(self.health_timeout)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    RobocubeError::TimeoutError(self.health_timeout.as_secs())
                } else if e.is_connect() {
                    RobocubeError::ConnectionError(format!(
                        "Failed to connect to server at {}: {}",
                        self.base_url, e
                    ))
                } else {
                    RobocubeError::RequestFailed(e)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RobocubeError::ServerError(format!(
                "Health check failed ({}): {}",
                status, error_text
            )));
        }

        response.json::<ServerStatus>().await.map_err(|e| {
            RobocubeError::ParseError(format!("Failed to parse health response: {}", e))
        })
    }

    /// Generate a 3D mesh from a text prompt
    ///
    /// This method sends a generation request to the Cube3D server and returns
    /// the resulting mesh data. Includes automatic retry logic for transient failures.
    ///
    /// # Arguments
    ///
    /// * `request` - Generation request containing the prompt and parameters
    ///
    /// # Returns
    ///
    /// The generated mesh with vertices, faces, and optional colors/normals
    pub async fn generate(&self, request: &GenerationRequest) -> Result<RobocubeResult> {
        // Validate request before sending
        request.validate()?;
        self.generate_with_retry(request, 0).await
    }

    /// Generate occupancy field from a text prompt
    ///
    /// This method queries the Cube3D shape model's occupancy decoder directly,
    /// returning discrete voxel occupancy values instead of a mesh. This is more
    /// suitable for voxel-based applications as it avoids mesh-to-voxel conversion.
    ///
    /// # Arguments
    ///
    /// * `request` - Occupancy request containing the prompt and grid parameters
    ///
    /// # Returns
    ///
    /// The occupancy field with occupied voxel positions and optional raw logits
    ///
    /// # Example
    ///
    /// ```no_run
    /// use robocube::{RobocubeClient, OccupancyRequest};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = RobocubeClient::new("http://localhost:8642");
    ///
    ///     let request = OccupancyRequest::new("A wooden chair")
    ///         .with_grid_resolution(64)
    ///         .with_threshold(0.0);
    ///
    ///     let result = client.generate_occupancy(&request).await?;
    ///     println!("Generated {} occupied voxels", result.occupied_count());
    ///     Ok(())
    /// }
    /// ```
    pub async fn generate_occupancy(&self, request: &OccupancyRequest) -> Result<OccupancyResult> {
        // Validate request before sending
        request.validate()?;
        self.generate_occupancy_with_retry(request, 0).await
    }

    /// Internal method for occupancy generation with retry logic
    fn generate_occupancy_with_retry<'a>(
        &'a self,
        request: &'a OccupancyRequest,
        attempt: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OccupancyResult>> + Send + 'a>>
    {
        Box::pin(async move {
            let url = format!("{}/generate_occupancy", self.base_url);

            let response = self
                .client
                .post(&url)
                .json(request)
                .timeout(self.generate_timeout)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        return resp.json::<OccupancyResult>().await.map_err(|e| {
                            RobocubeError::ParseError(format!("Failed to parse response: {}", e))
                        });
                    }

                    // Get error details
                    let error_text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());

                    // Check for retryable errors
                    if self.should_retry(status.as_u16(), &error_text, attempt) {
                        let delay = self.calculate_backoff_delay(attempt);
                        tokio::time::sleep(delay).await;
                        return self
                            .generate_occupancy_with_retry(request, attempt + 1)
                            .await;
                    }

                    // Handle specific error cases
                    if status.as_u16() == 503 && error_text.contains("loading") {
                        return Err(RobocubeError::ModelsNotLoaded);
                    }

                    Err(RobocubeError::ServerError(format!(
                        "Occupancy generation failed ({}): {}",
                        status, error_text
                    )))
                }
                Err(e) => {
                    // Check for retryable connection errors
                    if self.should_retry_error(&e, attempt) {
                        let delay = self.calculate_backoff_delay(attempt);
                        tokio::time::sleep(delay).await;
                        return self
                            .generate_occupancy_with_retry(request, attempt + 1)
                            .await;
                    }

                    if e.is_timeout() {
                        Err(RobocubeError::TimeoutError(self.generate_timeout.as_secs()))
                    } else if e.is_connect() {
                        Err(RobocubeError::ConnectionError(format!(
                            "Failed to connect to server at {}: {}",
                            self.base_url, e
                        )))
                    } else {
                        Err(RobocubeError::RequestFailed(e))
                    }
                }
            }
        })
    }

    /// Internal method for generation with retry logic
    fn generate_with_retry<'a>(
        &'a self,
        request: &'a GenerationRequest,
        attempt: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<RobocubeResult>> + Send + 'a>>
    {
        Box::pin(async move {
            let url = format!("{}/generate", self.base_url);

            let response = self
                .client
                .post(&url)
                .json(request)
                .timeout(self.generate_timeout)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        return resp.json::<RobocubeResult>().await.map_err(|e| {
                            RobocubeError::ParseError(format!("Failed to parse response: {}", e))
                        });
                    }

                    // Get error details
                    let error_text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());

                    // Check for retryable errors
                    if self.should_retry(status.as_u16(), &error_text, attempt) {
                        let delay = self.calculate_backoff_delay(attempt);
                        tokio::time::sleep(delay).await;
                        return self.generate_with_retry(request, attempt + 1).await;
                    }

                    // Handle specific error cases
                    if status.as_u16() == 503 && error_text.contains("loading") {
                        return Err(RobocubeError::ModelsNotLoaded);
                    }

                    Err(RobocubeError::ServerError(format!(
                        "Generation failed ({}): {}",
                        status, error_text
                    )))
                }
                Err(e) => {
                    // Check for retryable connection errors
                    if self.should_retry_error(&e, attempt) {
                        let delay = self.calculate_backoff_delay(attempt);
                        tokio::time::sleep(delay).await;
                        return self.generate_with_retry(request, attempt + 1).await;
                    }

                    if e.is_timeout() {
                        Err(RobocubeError::TimeoutError(self.generate_timeout.as_secs()))
                    } else if e.is_connect() {
                        Err(RobocubeError::ConnectionError(format!(
                            "Failed to connect to server at {}: {}",
                            self.base_url, e
                        )))
                    } else {
                        Err(RobocubeError::RequestFailed(e))
                    }
                }
            }
        })
    }

    /// Determine if a response should trigger a retry
    fn should_retry(&self, status_code: u16, error_text: &str, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }

        // Retry on server errors (5xx)
        if status_code >= 500 {
            // Special case: 503 with "still loading" should retry
            if status_code == 503 && error_text.contains("loading") {
                return true;
            }
            // Retry on other 5xx errors
            return status_code != 501; // Don't retry "Not Implemented"
        }

        false
    }

    /// Determine if a request error should trigger a retry
    fn should_retry_error(&self, error: &reqwest::Error, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }

        // Retry on connection errors and timeouts
        error.is_connect() || error.is_timeout()
    }

    /// Calculate exponential backoff delay
    fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.base_delay_ms * 2u64.pow(attempt);
        // Cap at 30 seconds
        Duration::from_millis(delay_ms.min(30000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = RobocubeClient::new("http://localhost:8642");
        assert_eq!(client.base_url(), "http://localhost:8642");
    }

    #[test]
    fn test_client_trailing_slash() {
        let client = RobocubeClient::new("http://localhost:8642/");
        assert_eq!(client.base_url(), "http://localhost:8642");
    }

    #[test]
    fn test_client_builder() {
        let client = RobocubeClient::new("http://localhost:8642")
            .with_health_timeout(Duration::from_secs(10))
            .with_generate_timeout(Duration::from_secs(300))
            .with_max_retries(5)
            .with_base_delay_ms(2000);

        assert_eq!(client.health_timeout, Duration::from_secs(10));
        assert_eq!(client.generate_timeout, Duration::from_secs(300));
        assert_eq!(client.max_retries, 5);
        assert_eq!(client.base_delay_ms, 2000);
    }

    #[test]
    fn test_backoff_delay() {
        let client = RobocubeClient::new("http://localhost:8642").with_base_delay_ms(1000);

        assert_eq!(
            client.calculate_backoff_delay(0),
            Duration::from_millis(1000)
        );
        assert_eq!(
            client.calculate_backoff_delay(1),
            Duration::from_millis(2000)
        );
        assert_eq!(
            client.calculate_backoff_delay(2),
            Duration::from_millis(4000)
        );
        assert_eq!(
            client.calculate_backoff_delay(3),
            Duration::from_millis(8000)
        );

        // Should cap at 30 seconds
        assert_eq!(
            client.calculate_backoff_delay(10),
            Duration::from_millis(30000)
        );
    }

    #[test]
    fn test_should_retry() {
        let client = RobocubeClient::new("http://localhost:8642").with_max_retries(3);

        // Should retry on 503 with loading message
        assert!(client.should_retry(503, "Models still loading", 0));

        // Should retry on 500 server error
        assert!(client.should_retry(500, "Internal error", 0));

        // Should NOT retry on 501 Not Implemented
        assert!(!client.should_retry(501, "Not implemented", 0));

        // Should NOT retry on 400 client error
        assert!(!client.should_retry(400, "Bad request", 0));

        // Should NOT retry when max attempts reached
        assert!(!client.should_retry(503, "loading", 3));
    }

    #[test]
    fn test_default_server_url() {
        assert_eq!(DEFAULT_SERVER_URL, "http://localhost:8642");
    }
}
