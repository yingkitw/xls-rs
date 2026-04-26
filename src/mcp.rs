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
    AddChartCapability, AddSparklineCapability, ApplyFormulaCapability, CapabilityRegistry,
    ConditionalFormatCapability, ConvertCapability, FilterCapability, ListSheetsCapability,
    ReadAllSheetsCapability, ReadExcelCapability, SortCapability, WorkflowCapability,
    WriteStyledCapability,
};
use rmcp::handler::server::tool::ToolRouter;
use crate::capability_catalog;
use crate::mcp_enrichment::{mcp_error_data, McpErrorContext};

fn tool_error(context: &str, e: anyhow::Error, ctx: McpErrorContext) -> McpError {
    let full = format!("{context}: {e:#}");
    McpError {
        code: ErrorCode::INTERNAL_ERROR,
        message: Cow::from(full.clone()),
        data: Some(mcp_error_data(&full, ctx)),
    }
}

fn serde_err(e: serde_json::Error) -> McpError {
    let s = e.to_string();
    McpError {
        code: ErrorCode::INTERNAL_ERROR,
        message: Cow::from(s.clone()),
        data: Some(mcp_error_data(&s, McpErrorContext::default())),
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WriteStyledRequest {
    #[schemars(description = "Output file path (.xlsx)")]
    pub output: String,
    #[schemars(description = "2D array of string values")]
    pub data: Vec<Vec<String>>,
    #[schemars(description = "Sheet name (default: Sheet1)")]
    pub sheet_name: Option<String>,
    #[schemars(description = "Apply header styling to first row")]
    pub style_header: Option<bool>,
    #[schemars(description = "Freeze first row")]
    pub freeze_header: Option<bool>,
    #[schemars(description = "Enable auto-filter")]
    pub auto_filter: Option<bool>,
    #[schemars(description = "Auto-fit column widths")]
    pub auto_fit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AddChartRequest {
    #[schemars(description = "Output file path (.xlsx)")]
    pub output: String,
    #[schemars(description = "2D array of string values")]
    pub data: Vec<Vec<String>>,
    #[schemars(description = "Chart type: bar, column, line, area, pie, scatter, doughnut")]
    pub chart_type: Option<String>,
    #[schemars(description = "Chart title")]
    pub title: Option<String>,
    #[schemars(description = "Column index for category labels")]
    pub category_column: Option<i64>,
    #[schemars(description = "Column indices for values")]
    pub value_columns: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AddSparklineRequest {
    #[schemars(description = "Output file path (.xlsx)")]
    pub output: String,
    #[schemars(description = "Data range for sparkline (e.g., A1:A10)")]
    pub data_range: String,
    #[schemars(description = "Cell to place sparkline (e.g., B1)")]
    pub sparkline_cell: String,
    #[schemars(description = "Sheet name (default: Sheet1)")]
    pub sheet_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConditionalFormatRequest {
    #[schemars(description = "Output file path (.xlsx)")]
    pub output: String,
    #[schemars(description = "Range to format (e.g., A1:B10)")]
    pub range: String,
    #[schemars(description = "Formula condition (e.g., '=A1>100')")]
    pub condition: String,
    #[schemars(description = "Background color hex (e.g., 'FF0000')")]
    pub bg_color: Option<String>,
    #[schemars(description = "Font color hex (e.g., 'FFFFFF')")]
    pub font_color: Option<String>,
    #[schemars(description = "Bold text")]
    pub bold: Option<bool>,
    #[schemars(description = "Sheet name (default: Sheet1)")]
    pub sheet_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ListSheetsRequest {
    #[schemars(description = "Input file path (.xlsx, .xls, .ods)")]
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReadExcelRequest {
    #[schemars(description = "Input file path (.xlsx, .xls, .ods)")]
    pub input: String,
    #[schemars(description = "Sheet name (default: first sheet)")]
    pub sheet: Option<String>,
    #[schemars(description = "Cell range in A1 notation (e.g., A1:B10)")]
    pub range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReadAllSheetsRequest {
    #[schemars(description = "Input file path (.xlsx, .xls, .ods)")]
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ApplyFormulaRequest {
    #[schemars(description = "Input file path")]
    pub input: String,
    #[schemars(description = "Output file path")]
    pub output: String,
    #[schemars(description = "Formula to apply")]
    pub formula: String,
    #[schemars(description = "Target cell (e.g., B1)")]
    pub cell: Option<String>,
    #[schemars(description = "Target range (e.g., B1:B10)")]
    pub range: Option<String>,
    #[schemars(description = "Sheet name for Excel files")]
    pub sheet: Option<String>,
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

        // Register Excel read capabilities
        registry.register(Arc::new(ListSheetsCapability));
        registry.register(Arc::new(ReadExcelCapability));
        registry.register(Arc::new(ReadAllSheetsCapability));

        // Register Excel write capabilities
        registry.register(Arc::new(WriteStyledCapability));
        registry.register(Arc::new(AddChartCapability));
        registry.register(Arc::new(AddSparklineCapability));
        registry.register(Arc::new(ConditionalFormatCapability));

        // Register formula capabilities
        registry.register(Arc::new(ApplyFormulaCapability));

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
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.input.clone()),
            input: Some(request.0.input.clone()),
            output: Some(request.0.output.clone()),
            ..Default::default()
        };
        match self.registry.execute("sort", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to sort", e, ctx)),
        }
    }

    #[tool(description = "Convert a spreadsheet file to another format (csv, xlsx, parquet, avro, ods, …)")]
    async fn convert_data(
        &self,
        request: Parameters<ConvertRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.input.clone()),
            input: Some(request.0.input.clone()),
            output: Some(request.0.output.clone()),
            sheet: request.0.sheet.clone(),
            ..Default::default()
        };
        match self.registry.execute("convert", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to convert", e, ctx)),
        }
    }

    #[tool(description = "Filter rows based on a condition")]
    async fn filter_data(
        &self,
        request: Parameters<FilterRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.input.clone()),
            input: Some(request.0.input.clone()),
            output: Some(request.0.output.clone()),
            ..Default::default()
        };
        match self.registry.execute("filter", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to filter", e, ctx)),
        }
    }

    #[tool(description = "Execute a data processing workflow from a JSON plan")]
    async fn execute_workflow(
        &self,
        request: Parameters<ExecuteWorkflowRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        match self.registry.execute("execute_workflow", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to execute workflow", e, McpErrorContext::default())),
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

    #[tool(description = "Write data to Excel with styling options")]
    async fn write_styled(
        &self,
        request: Parameters<WriteStyledRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.output.clone()),
            output: Some(request.0.output.clone()),
            sheet: request.0.sheet_name.clone(),
            ..Default::default()
        };
        match self.registry.execute("write_styled", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to write styled", e, ctx)),
        }
    }

    #[tool(description = "Write data to Excel with an embedded chart")]
    async fn add_chart(
        &self,
        request: Parameters<AddChartRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.output.clone()),
            output: Some(request.0.output.clone()),
            ..Default::default()
        };
        match self.registry.execute("add_chart", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to add chart", e, ctx)),
        }
    }

    #[tool(description = "Add a sparkline to an Excel file")]
    async fn add_sparkline(
        &self,
        request: Parameters<AddSparklineRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.output.clone()),
            output: Some(request.0.output.clone()),
            range: Some(request.0.data_range.clone()),
            cell: Some(request.0.sparkline_cell.clone()),
            sheet: request.0.sheet_name.clone(),
            ..Default::default()
        };
        match self.registry.execute("add_sparkline", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to add sparkline", e, ctx)),
        }
    }

    #[tool(description = "Apply conditional formatting to an Excel range")]
    async fn conditional_format(
        &self,
        request: Parameters<ConditionalFormatRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.output.clone()),
            output: Some(request.0.output.clone()),
            range: Some(request.0.range.clone()),
            sheet: request.0.sheet_name.clone(),
            ..Default::default()
        };
        match self.registry.execute("conditional_format", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to apply conditional format", e, ctx)),
        }
    }

    #[tool(description = "List all sheet names in an Excel workbook")]
    async fn list_sheets(
        &self,
        request: Parameters<ListSheetsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.input.clone()),
            input: Some(request.0.input.clone()),
            ..Default::default()
        };
        match self.registry.execute("list_sheets", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to list sheets", e, ctx)),
        }
    }

    #[tool(description = "Read data from an Excel file with optional sheet and range selection")]
    async fn read_excel(
        &self,
        request: Parameters<ReadExcelRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.input.clone()),
            input: Some(request.0.input.clone()),
            sheet: request.0.sheet.clone(),
            range: request.0.range.clone(),
            ..Default::default()
        };
        match self.registry.execute("read_excel", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to read Excel", e, ctx)),
        }
    }

    #[tool(description = "Read data from all sheets in an Excel workbook")]
    async fn read_all_sheets(
        &self,
        request: Parameters<ReadAllSheetsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.input.clone()),
            input: Some(request.0.input.clone()),
            ..Default::default()
        };
        match self.registry.execute("read_all_sheets", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to read all sheets", e, ctx)),
        }
    }

    #[tool(description = "Apply a formula to a cell or range in a spreadsheet")]
    async fn apply_formula(
        &self,
        request: Parameters<ApplyFormulaRequest>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::to_value(&request.0).map_err(serde_err)?;
        let ctx = McpErrorContext {
            file: Some(request.0.input.clone()),
            input: Some(request.0.input.clone()),
            output: Some(request.0.output.clone()),
            sheet: request.0.sheet.clone(),
            range: request.0.range.clone(),
            cell: request.0.cell.clone(),
            ..Default::default()
        };
        match self.registry.execute("apply_formula", args) {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result.to_string())])),
            Err(e) => Err(tool_error("Failed to apply formula", e, ctx)),
        }
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
