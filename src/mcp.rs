use rmcp::{
    ServerHandler,
    handler::server::wrapper::Parameters,
    model::{ErrorData as McpError, *},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

use crate::capabilities::{
    CapabilityRegistry, ConvertCapability, FilterCapability, SortCapability, WorkflowCapability,
};
use rmcp::handler::server::tool::ToolRouter;
use crate::capability_catalog;

#[derive(Clone)]
pub struct XlsRsMcpServer {
    tool_router: ToolRouter<XlsRsMcpServer>,
    registry: Arc<CapabilityRegistry>,
}

// Manually define requests for now until we can bridge schema
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SortRequest {
    #[schemars(description = "Input file path")]
    pub input: String,
    #[schemars(description = "Output file path")]
    pub output: String,
    #[schemars(description = "Column name or index to sort by")]
    pub column: String,
    #[schemars(description = "Sort in ascending order (default: true)")]
    pub ascending: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConvertRequest {
    #[schemars(description = "Input file path")]
    pub input: String,
    #[schemars(description = "Output file path")]
    pub output: String,
    #[schemars(description = "Optional sheet name when reading Excel or ODS")]
    pub sheet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FilterRequest {
    #[schemars(description = "Input file path")]
    pub input: String,
    #[schemars(description = "Output file path")]
    pub output: String,
    #[schemars(description = "Column to filter on")]
    pub column: String,
    #[schemars(description = "Operator: =, !=, >, >=, <, <=, contains, starts_with, ends_with, regex")]
    pub operator: String,
    #[schemars(description = "Value to compare against")]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExecuteWorkflowRequest {
    #[schemars(description = "Workflow configuration object (JSON)")]
    pub workflow: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CapabilitiesRequest {}

fn make_error(msg: String) -> McpError {
    let detail = msg.clone();
    McpError {
        code: ErrorCode::INTERNAL_ERROR,
        message: Cow::from(msg),
        data: Some(serde_json::json!({
            "kind": "xls_rs_error",
            "detail": detail,
        })),
    }
}

#[tool_router]
impl XlsRsMcpServer {
    pub fn new() -> Self {
        let registry = Arc::new(CapabilityRegistry::new());
        
        // Register capabilities
        registry.register(Arc::new(SortCapability));
        registry.register(Arc::new(FilterCapability));
        registry.register(Arc::new(ConvertCapability));
        registry.register(Arc::new(WorkflowCapability::new()));
        
        Self {
            tool_router: Self::tool_router(),
            registry,
        }
    }

    #[tool(description = "Sort data by a specific column")]
    async fn sort_data(
        &self,
        request: Parameters<SortRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(|e| make_error(e.to_string()))?;
        
        match self.registry.execute("sort", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(make_error(format!("Failed to sort: {}", e))),
        }
    }

    #[tool(description = "Convert a spreadsheet file to another format (csv, xlsx, parquet, avro, ods, …)")]
    async fn convert_data(
        &self,
        request: Parameters<ConvertRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(|e| make_error(e.to_string()))?;
        match self.registry.execute("convert", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(make_error(format!("Failed to convert: {}", e))),
        }
    }

    #[tool(description = "Filter rows based on a condition")]
    async fn filter_data(
        &self,
        request: Parameters<FilterRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(|e| make_error(e.to_string()))?;
        
        match self.registry.execute("filter", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(make_error(format!("Failed to filter: {}", e))),
        }
    }

    #[tool(description = "Execute a data processing workflow from a JSON plan")]
    async fn execute_workflow(
        &self,
        request: Parameters<ExecuteWorkflowRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(|e| make_error(e.to_string()))?;
        
        match self.registry.execute("execute_workflow", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(make_error(format!("Failed to execute workflow: {}", e))),
        }
    }

    #[tool(description = "List supported capabilities and formats")]
    async fn capabilities(
        &self,
        _request: Parameters<CapabilitiesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let caps: Vec<serde_json::Value> = capability_catalog::CAPABILITIES
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "kind": format!("{:?}", c.kind),
                })
            })
            .collect();

        let formats = capability_catalog::FORMATS;

        let payload = serde_json::json!({
            "capabilities": caps,
            "formats": formats,
        });

        Ok(CallToolResult::success(vec![Content::text(
            payload.to_string(),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for XlsRsMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "A spreadsheet tool for reading, writing, converting CSV and Excel files with formula support. \
                Use convert_data to change formats, sort_data / filter_data for row operations, and execute_workflow for pipelines."
                    .to_string(),
            ),
        }
    }
}
