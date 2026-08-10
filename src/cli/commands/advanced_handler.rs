//! Advanced command handlers

use crate::cli::commands::advanced;
use anyhow::Result;

/// Advanced command handler
#[derive(Default)]
pub struct AdvancedCommandHandler;

impl AdvancedCommandHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_profile(&self, input: String, output: Option<String>) -> Result<()> {
        advanced::handle_profile(input, output)
    }

    pub fn handle_schema(&self, input: String, output: Option<String>) -> Result<()> {
        advanced::handle_schema(input, output)
    }

    pub fn handle_to_sql(
        &self,
        input: String,
        table: String,
        output: Option<String>,
        batch_size: Option<usize>,
    ) -> Result<()> {
        advanced::handle_to_sql(input, table, output, batch_size)
    }

    pub fn handle_validate(
        &self,
        input: String,
        rules: String,
        output: Option<String>,
        report: Option<String>,
    ) -> Result<()> {
        advanced::handle_validate(input, rules, output, report)
    }

    pub fn handle_chart(
        &self,
        input: String,
        output: String,
        chart_type: String,
        title: Option<String>,
        x_column: Option<String>,
        y_column: Option<String>,
    ) -> Result<()> {
        advanced::handle_chart(input, output, chart_type, title, x_column, y_column)
    }

    pub fn handle_config_init(&self) -> Result<()> {
        advanced::handle_config_init()
    }

    pub fn handle_export_styled(
        &self,
        input: String,
        output: String,
        style: Option<String>,
    ) -> Result<()> {
        advanced::handle_export_styled(input, output, style)
    }

    pub fn handle_examples_generate(&self) -> Result<()> {
        advanced::handle_examples_generate()
    }

    pub fn handle_add_chart(
        &self,
        input: String,
        output: String,
        chart_type: String,
        title: Option<String>,
        category_column: Option<usize>,
        value_columns: Option<Vec<usize>>,
    ) -> Result<()> {
        advanced::handle_add_chart(input, output, chart_type, title, category_column, value_columns)
    }

    pub fn handle_add_sparkline(
        &self,
        output: String,
        data_range: String,
        sparkline_cell: String,
        sheet: Option<String>,
    ) -> Result<()> {
        advanced::handle_add_sparkline(output, data_range, sparkline_cell, sheet)
    }

    pub fn handle_conditional_format(
        &self,
        output: String,
        range: String,
        condition: String,
        bg_color: Option<String>,
        font_color: Option<String>,
        bold: Option<bool>,
        sheet: Option<String>,
    ) -> Result<()> {
        advanced::handle_conditional_format(
            output,
            range,
            condition,
            bg_color,
            font_color,
            bold,
            sheet,
        )
    }

    pub fn handle_apply_formula_range(
        &self,
        input: String,
        output: String,
        formula: String,
        range: String,
        sheet: Option<String>,
    ) -> Result<()> {
        advanced::handle_apply_formula_range(input, output, formula, range, sheet)
    }
}
