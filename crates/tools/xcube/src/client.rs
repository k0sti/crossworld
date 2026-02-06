//! XCube inference server HTTP client implementation

use crate::types::{GenerationRequest, ServerStatus, XCubeError, XCubeResult};
use reqwest::Client;
use std::time::Duration;

/// Default timeout for health check requests (5 seconds)
const DEFAULT_HEALTH_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for generation requests (5 minutes)
const DEFAULT_GENERATE_TIMEOUT: Duration = Duration::from_secs(300);

/// Default number of retry attempts for transient failures
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default base delay for exponential backoff (1 second)
const DEFAULT_BASE_DELAY_MS: u64 = 1000;

/// XCube inference server HTTP client
///
/// This client communicates with the XCube Python inference server
/// to generate 3D point clouds from text prompts using the XCube
/// diffusion model.
///
/// # Example
///
/// ```no_run
/// use xcube::{XCubeClient, GenerationRequest};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = XCubeClient::new("http://localhost:8000");
///
///     // Check server health
///     let status = client.health_check().await?;
///     if !status.is_ready() {
///         println!("Server not ready: {}", status.status);
///         return Ok(());
///     }
///
///     // Generate a 3D model
///     let request = GenerationRequest::new("a wooden chair")
///         .with_ddim_steps(50)
///         .with_fine(false);
///
///     let result = client.generate(&request).await?;
///     println!("Generated {} coarse points", result.coarse_point_count());
///
///     Ok(())
/// }
/// ```
pub struct XCubeClient {
    client: Client,
    base_url: String,
    health_timeout: Duration,
    generate_timeout: Duration,
    max_retries: u32,
    base_delay_ms: u64,
}

impl XCubeClient {
    /// Create a new XCube client with the given server URL
    ///
    /// # Arguments
    ///
    /// * `server_url` - Base URL of the XCube inference server (e.g., "http://localhost:8000")
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: server_url.into().trim_end_matches('/').to_string(),
            health_timeout: DEFAULT_HEALTH_TIMEOUT,
            generate_timeout: DEFAULT_GENERATE_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay_ms: DEFAULT_BASE_DELAY_MS,
        }
    }

    /// Set the timeout for health check requests
    pub fn with_health_timeout(mut self, timeout: Duration) -> Self {
        self.health_timeout = timeout;
        self
    }

    /// Set the timeout for generation requests
    pub fn with_generate_timeout(mut self, timeout: Duration) -> Self {
        self.generate_timeout = timeout;
        self
    }

    /// Set the maximum number of retry attempts for transient failures
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the base delay for exponential backoff (in milliseconds)
    pub fn with_base_delay_ms(mut self, delay_ms: u64) -> Self {
        self.base_delay_ms = delay_ms;
        self
    }

    /// Check the health status of the XCube server
    ///
    /// This endpoint returns information about the server's readiness,
    /// GPU availability, and whether models are loaded.
    ///
    /// # Returns
    ///
    /// Returns `Ok(ServerStatus)` with server status information, or an error
    /// if the request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xcube::XCubeClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = XCubeClient::new("http://localhost:8000");
    /// let status = client.health_check().await?;
    ///
    /// if status.is_ready() {
    ///     println!("Server ready with GPU: {}", status.gpu_name.unwrap_or_default());
    /// } else if status.is_loading() {
    ///     println!("Server is loading models...");
    /// } else {
    ///     println!("Server error: {:?}", status.error);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check(&self) -> Result<ServerStatus, XCubeError> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(self.health_timeout)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    XCubeError::TimeoutError(self.health_timeout.as_secs())
                } else if e.is_connect() {
                    XCubeError::ConnectionError(format!("Failed to connect to {}", self.base_url))
                } else {
                    XCubeError::RequestFailed(e)
                }
            })?;

        if !response.status().is_success() {
            return Err(XCubeError::ServerError(format!(
                "Health check failed with status: {}",
                response.status()
            )));
        }

        response
            .json::<ServerStatus>()
            .await
            .map_err(|e| XCubeError::ParseError(format!("Failed to parse health response: {}", e)))
    }

    /// Generate a 3D point cloud from a text prompt
    ///
    /// This method sends a generation request to the XCube server and waits
    /// for the inference to complete. Generation typically takes 10-60 seconds
    /// depending on the configuration and hardware.
    ///
    /// The method includes automatic retry logic with exponential backoff for
    /// transient failures (connection errors, 5xx errors).
    ///
    /// # Arguments
    ///
    /// * `request` - Generation request containing the prompt and parameters
    ///
    /// # Returns
    ///
    /// Returns `Ok(XCubeResult)` with the generated point cloud data, or an error
    /// if the request fails or times out.
    ///
    /// # Errors
    ///
    /// - `XCubeError::ModelsNotLoaded` - Server models are not loaded yet
    /// - `XCubeError::TimeoutError` - Request exceeded timeout duration
    /// - `XCubeError::ConnectionError` - Failed to connect to server
    /// - `XCubeError::ServerError` - Server returned an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xcube::{XCubeClient, GenerationRequest};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = XCubeClient::new("http://localhost:8000");
    ///
    /// let request = GenerationRequest::new("a red sports car")
    ///     .with_ddim_steps(100)
    ///     .with_guidance_scale(7.5)
    ///     .with_seed(42)
    ///     .with_fine(true);
    ///
    /// let result = client.generate(&request).await?;
    ///
    /// println!("Coarse points: {}", result.coarse_point_count());
    /// if result.has_fine() {
    ///     println!("Fine points: {}", result.fine_point_count());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate(&self, request: &GenerationRequest) -> Result<XCubeResult, XCubeError> {
        self.generate_with_retry(request, 0).await
    }

    /// Internal method to generate with retry logic
    fn generate_with_retry<'a>(
        &'a self,
        request: &'a GenerationRequest,
        attempt: u32,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<XCubeResult, XCubeError>> + Send + 'a>,
    > {
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

                    // Check for server-side errors
                    if status.is_server_error() {
                        // 503 Service Unavailable - models not loaded
                        if status.as_u16() == 503 {
                            let error_text = resp
                                .text()
                                .await
                                .unwrap_or_else(|_| "Models not loaded".to_string());

                            // Retry if models are still loading
                            if error_text.contains("still loading") && attempt < self.max_retries {
                                let delay = self.calculate_backoff_delay(attempt);
                                tokio::time::sleep(delay).await;
                                return self.generate_with_retry(request, attempt + 1).await;
                            }

                            return Err(XCubeError::ModelsNotLoaded);
                        }

                        // Other 5xx errors - retry if possible
                        if attempt < self.max_retries {
                            let delay = self.calculate_backoff_delay(attempt);
                            tokio::time::sleep(delay).await;
                            return self.generate_with_retry(request, attempt + 1).await;
                        }

                        let error_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Server error".to_string());
                        return Err(XCubeError::ServerError(error_text));
                    }

                    // Check for client errors (4xx)
                    if status.is_client_error() {
                        let error_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Bad request".to_string());
                        return Err(XCubeError::ServerError(format!(
                            "Client error ({}): {}",
                            status, error_text
                        )));
                    }

                    // Success - parse result
                    resp.json::<XCubeResult>().await.map_err(|e| {
                        XCubeError::ParseError(format!(
                            "Failed to parse generation response: {}",
                            e
                        ))
                    })
                }
                Err(e) => {
                    // Handle timeout
                    if e.is_timeout() {
                        return Err(XCubeError::TimeoutError(self.generate_timeout.as_secs()));
                    }

                    // Handle connection errors with retry
                    if e.is_connect() && attempt < self.max_retries {
                        let delay = self.calculate_backoff_delay(attempt);
                        tokio::time::sleep(delay).await;
                        return self.generate_with_retry(request, attempt + 1).await;
                    }

                    if e.is_connect() {
                        return Err(XCubeError::ConnectionError(format!(
                            "Failed to connect to {} after {} attempts",
                            self.base_url,
                            attempt + 1
                        )));
                    }

                    Err(XCubeError::RequestFailed(e))
                }
            }
        })
    }

    /// Calculate exponential backoff delay for retry attempts
    fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.base_delay_ms * 2u64.pow(attempt);
        Duration::from_millis(delay_ms)
    }
}

impl Default for XCubeClient {
    /// Create a client with default localhost URL
    fn default() -> Self {
        Self::new("http://localhost:8000")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = XCubeClient::new("http://localhost:8000");
        assert_eq!(client.base_url, "http://localhost:8000");
        assert_eq!(client.health_timeout, DEFAULT_HEALTH_TIMEOUT);
        assert_eq!(client.generate_timeout, DEFAULT_GENERATE_TIMEOUT);
        assert_eq!(client.max_retries, DEFAULT_MAX_RETRIES);
    }

    #[test]
    fn test_client_with_trailing_slash() {
        let client = XCubeClient::new("http://localhost:8000/");
        assert_eq!(client.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_client_configuration() {
        let client = XCubeClient::new("http://localhost:8000")
            .with_health_timeout(Duration::from_secs(10))
            .with_generate_timeout(Duration::from_secs(600))
            .with_max_retries(5)
            .with_base_delay_ms(2000);

        assert_eq!(client.health_timeout, Duration::from_secs(10));
        assert_eq!(client.generate_timeout, Duration::from_secs(600));
        assert_eq!(client.max_retries, 5);
        assert_eq!(client.base_delay_ms, 2000);
    }

    #[test]
    fn test_backoff_delay_calculation() {
        let client = XCubeClient::new("http://localhost:8000");

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
    }

    #[test]
    fn test_default_client() {
        let client = XCubeClient::default();
        assert_eq!(client.base_url, "http://localhost:8000");
    }
}
