//! REST API server mode
//!
//! Provides HTTP API endpoints for xls-rs operations using axum.
//!
//! # Example
//!
//! ```no_run
//! use xls_rs::api::{ApiConfig, ApiServer};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = ApiConfig::default();
//!     let server = ApiServer::new(config);
//!     server.start().await?;
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[cfg(feature = "api")]
use axum::{
    extract::{DefaultBodyLimit, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
#[cfg(feature = "api")]
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
};

#[cfg(feature = "api")]
use anyhow::Context;

/// API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub cors_enabled: bool,
    pub max_request_size: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            cors_enabled: true,
            max_request_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// API request types
#[derive(Debug, Deserialize)]
#[serde(tag = "operation")]
pub enum ApiRequest {
    Read {
        input: String,
        sheet: Option<String>,
        range: Option<String>,
    },
    Write {
        output: String,
        data: Vec<Vec<String>>,
        sheet: Option<String>,
    },
    Convert {
        input: String,
        output: String,
        sheet: Option<String>,
    },
    Profile {
        input: String,
        sample_size: Option<usize>,
    },
    Validate {
        input: String,
        rules: String,
    },
    Filter {
        input: String,
        where_clause: String,
    },
    Sort {
        input: String,
        column: String,
        ascending: bool,
    },
}

/// API response
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub message: Option<String>,
}

impl ApiResponse {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            message: None,
        }
    }

    pub fn message(message: String) -> Self {
        Self {
            success: true,
            data: None,
            error: None,
            message: Some(message),
        }
    }
}

/// API server
pub struct ApiServer {
    config: ApiConfig,
}

impl ApiServer {
    pub fn new(config: ApiConfig) -> Self {
        Self { config }
    }

    /// Start the API server (requires the "api" feature)
    #[cfg(feature = "api")]
    pub async fn start(&self) -> Result<()> {
        // Build our application with routes
        let app = Router::new()
            .route("/api/read", post(handle_read))
            .route("/api/write", post(handle_write))
            .route("/api/convert", post(handle_convert))
            .route("/api/profile", post(handle_profile))
            .route("/api/validate", post(handle_validate))
            .route("/api/filter", post(handle_filter))
            .route("/api/sort", post(handle_sort))
            .layer(DefaultBodyLimit::max(self.config.max_request_size))
            .layer(RequestBodyLimitLayer::new(self.config.max_request_size));

        let app = if self.config.cors_enabled {
            app.layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        } else {
            app
        };

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind to {addr}"))?;

        println!("🚀 API server listening on http://{}", addr);
        println!("📊 Available endpoints:");
        println!("   POST /api/read      - Read data from a file");
        println!("   POST /api/write     - Write data to a file");
        println!("   POST /api/convert   - Convert between file formats");
        println!("   POST /api/profile   - Generate data profile");
        println!("   POST /api/validate  - Validate data against rules");
        println!("   POST /api/filter    - Filter data rows");
        println!("   POST /api/sort      - Sort data by column");

        axum::serve(listener, app).await.context("API server error")?;

        Ok(())
    }

    /// Start the API server (fallback when "api" feature is not enabled)
    #[cfg(not(feature = "api"))]
    pub async fn start(&self) -> Result<()> {
        use anyhow::bail;
        bail!(
            "API server is not enabled. Please rebuild with the 'api' feature: cargo build --features api"
        )
    }
}

/// Error response type
#[cfg(feature = "api")]
struct ApiError(anyhow::Error);

#[cfg(feature = "api")]
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ApiResponse::error(self.0.to_string()));
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

/// Handler for /api/read
#[cfg(feature = "api")]
async fn handle_read(Json(req): Json<ApiRequest>) -> Result<Json<ApiResponse>, ApiError> {
    use crate::converter::Converter;
    use crate::csv_handler::CellRange;
    use crate::helpers::filter_by_range;

    let converter = Converter::new();

    let (input, sheet, range) = match req {
        ApiRequest::Read { input, sheet, range } => (input, sheet, range),
        _ => return Err(ApiError(anyhow::anyhow!("Invalid request"))),
    };

    let mut data = converter
        .read_any_data(&input, sheet.as_deref())
        .map_err(ApiError)?;

    if let Some(ref range_str) = range {
        let cell = CellRange::parse(range_str).map_err(ApiError)?;
        data = filter_by_range(&data, &cell);
    }

    let response = ApiResponse::success(serde_json::json!({ "data": data }));

    Ok(Json(response))
}

/// Handler for /api/write
#[cfg(feature = "api")]
async fn handle_write(Json(req): Json<ApiRequest>) -> Result<Json<ApiResponse>, ApiError> {
    use crate::traits::{DataWriteOptions, DataWriter};

    let (output, data, sheet) = match req {
        ApiRequest::Write { output, data, sheet } => (output, data, sheet),
        _ => return Err(ApiError(anyhow::anyhow!("Invalid request"))),
    };

    // Determine file format from extension
    let format = output
        .rsplit('.')
        .next()
        .ok_or_else(|| ApiError(anyhow::anyhow!("Invalid file path")))?;

    let options = DataWriteOptions {
        sheet_name: sheet,
        column_names: None,
        include_headers: true,
    };

    match format {
        "csv" => {
            use crate::csv_handler::CsvHandler;
            let handler = CsvHandler::new();
            handler
                .write(&output, &data, options)
                .map_err(ApiError)?;
        }
        "xlsx" => {
            use crate::excel::ExcelHandler;
            let handler = ExcelHandler::new();
            handler
                .write(&output, &data, options)
                .map_err(ApiError)?;
        }
        "parquet" => {
            use crate::columnar::ParquetHandler;
            let handler = ParquetHandler::new();
            let (col_names, body): (Option<&[String]>, &[Vec<String>]) =
                if options.include_headers && !data.is_empty() {
                    (Some(&data[0]), data.get(1..).unwrap_or_default())
                } else {
                    (None, &data)
                };
            if body.is_empty() {
                return Err(ApiError(anyhow::anyhow!(
                    "Cannot write empty data to Parquet"
                )));
            }
            handler
                .write(&output, body, col_names)
                .map_err(ApiError)?;
        }
        "avro" => {
            use crate::columnar::AvroHandler;
            let handler = AvroHandler::new();
            let (field_names, body): (Option<&[String]>, &[Vec<String>]) =
                if options.include_headers && !data.is_empty() {
                    (Some(&data[0]), data.get(1..).unwrap_or_default())
                } else {
                    (None, &data)
                };
            if body.is_empty() {
                return Err(ApiError(anyhow::anyhow!("Cannot write empty data to Avro")));
            }
            handler
                .write(&output, body, field_names)
                .map_err(ApiError)?;
        }
        _ => {
            return Err(ApiError(anyhow::anyhow!(
                "Unsupported output format: {}",
                format
            )))
        }
    }

    Ok(Json(ApiResponse::message(format!(
        "Data written to {}",
        output
    ))))
}

/// Handler for /api/convert
#[cfg(feature = "api")]
async fn handle_convert(Json(req): Json<ApiRequest>) -> Result<Json<ApiResponse>, ApiError> {
    use crate::converter::Converter;

    let (input, output, sheet) = match req {
        ApiRequest::Convert { input, output, sheet } => (input, output, sheet),
        _ => return Err(ApiError(anyhow::anyhow!("Invalid request"))),
    };

    let converter = Converter::new();
    converter
        .convert(&input, &output, sheet.as_deref())
        .map_err(ApiError)?;

    Ok(Json(ApiResponse::message(format!(
        "Converted {} to {}",
        input, output
    ))))
}

/// Handler for /api/profile
#[cfg(feature = "api")]
async fn handle_profile(Json(req): Json<ApiRequest>) -> Result<Json<ApiResponse>, ApiError> {
    use crate::converter::Converter;
    use crate::profiling::DataProfiler;

    let (input, sample_size) = match req {
        ApiRequest::Profile { input, sample_size } => (input, sample_size),
        _ => return Err(ApiError(anyhow::anyhow!("Invalid request"))),
    };

    let converter = Converter::new();
    let data = converter
        .read_any_data(&input, None)
        .map_err(ApiError)?;

    let mut profiler = DataProfiler::new();
    if let Some(size) = sample_size {
        profiler = profiler.with_sample_size(size);
    }

    let profile = profiler.profile(&data, &input).map_err(ApiError)?;

    let value = serde_json::to_value(profile).map_err(|e| {
        ApiError(anyhow::anyhow!("Failed to serialize profile: {}", e))
    })?;

    Ok(Json(ApiResponse::success(value)))
}

/// Handler for /api/validate
#[cfg(feature = "api")]
async fn handle_validate(Json(req): Json<ApiRequest>) -> Result<Json<ApiResponse>, ApiError> {
    use crate::converter::Converter;
    use crate::validation::{DataValidator, ValidationConfig};

    let (input, rules) = match req {
        ApiRequest::Validate { input, rules } => (input, rules),
        _ => return Err(ApiError(anyhow::anyhow!("Invalid request"))),
    };

    let converter = Converter::new();
    let data = converter
        .read_any_data(&input, None)
        .map_err(ApiError)?;

    let config: ValidationConfig = serde_json::from_str(&rules)
        .map_err(|e| ApiError(anyhow::anyhow!("Invalid validation config JSON: {}", e)))?;

    let validator = DataValidator::new(config);
    let result = validator.validate(&data).map_err(ApiError)?;

    let value = serde_json::to_value(result)
        .map_err(|e| ApiError(anyhow::anyhow!("Failed to serialize validation result: {}", e)))?;

    Ok(Json(ApiResponse::success(value)))
}

/// Handler for /api/filter
#[cfg(feature = "api")]
async fn handle_filter(Json(req): Json<ApiRequest>) -> Result<Json<ApiResponse>, ApiError> {
    use crate::converter::Converter;
    use crate::operations::DataOperations;

    let (input, where_clause) = match req {
        ApiRequest::Filter {
            input,
            where_clause,
        } => (input, where_clause),
        _ => return Err(ApiError(anyhow::anyhow!("Invalid request"))),
    };

    let converter = Converter::new();
    let data = converter.read_any_data(&input, None).map_err(ApiError)?;

    let ops = DataOperations::new();
    let filtered = ops.query(&data, &where_clause).map_err(ApiError)?;

    Ok(Json(ApiResponse::success(serde_json::json!({ "data": filtered }))))
}

/// Handler for /api/sort
#[cfg(feature = "api")]
async fn handle_sort(Json(req): Json<ApiRequest>) -> Result<Json<ApiResponse>, ApiError> {
    use crate::converter::Converter;
    use crate::operations::DataOperations;
    use crate::traits::SortOperator;

    let (input, column, ascending) = match req {
        ApiRequest::Sort {
            input,
            column,
            ascending,
        } => (input, column, ascending),
        _ => return Err(ApiError(anyhow::anyhow!("Invalid request"))),
    };

    let converter = Converter::new();
    let mut data = converter.read_any_data(&input, None).map_err(ApiError)?;

    let ops = DataOperations::new();

    // Find column index by name
    if data.is_empty() {
        return Err(ApiError(anyhow::anyhow!("Data is empty")));
    }

    let column_idx = data[0]
        .iter()
        .position(|c| c == &column)
        .ok_or_else(|| ApiError(anyhow::anyhow!("Column '{}' not found", column)))?;

    ops.sort(&mut data, column_idx, ascending).map_err(ApiError)?;

    Ok(Json(ApiResponse::success(serde_json::json!({ "data": data }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.cors_enabled);
        assert_eq!(config.max_request_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success(serde_json::json!({"test": "data"}));
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::error("Test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_api_response_message() {
        let response = ApiResponse::message("Test message".to_string());
        assert!(response.success);
        assert!(response.message.is_some());
        assert_eq!(response.message.unwrap(), "Test message");
    }
}
