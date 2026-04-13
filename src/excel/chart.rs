use anyhow::Result;

use super::reader::ExcelHandler;

/// Chart type for visualization
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataChartType {
    Bar,
    Column,
    Line,
    Area,
    Pie,
    Scatter,
    Doughnut,
}

impl DataChartType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "bar" => Ok(DataChartType::Bar),
            "column" => Ok(DataChartType::Column),
            "line" => Ok(DataChartType::Line),
            "area" => Ok(DataChartType::Area),
            "pie" => Ok(DataChartType::Pie),
            "scatter" => Ok(DataChartType::Scatter),
            "doughnut" | "donut" => Ok(DataChartType::Doughnut),
            _ => anyhow::bail!(
                "Unknown chart type: {}. Use: bar, column, line, area, pie, scatter, doughnut",
                s
            ),
        }
    }
}

/// Chart configuration
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub chart_type: DataChartType,
    pub title: Option<String>,
    pub x_axis_title: Option<String>,
    pub y_axis_title: Option<String>,
    pub category_column: usize,
    pub value_columns: Vec<usize>,
    pub width: u32,
    pub height: u32,
    pub show_legend: bool,
    pub colors: Option<Vec<String>>,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            chart_type: DataChartType::Column,
            title: None,
            x_axis_title: None,
            y_axis_title: None,
            category_column: 0,
            value_columns: vec![1],
            width: 600,
            height: 400,
            show_legend: true,
            colors: None,
        }
    }
}

impl ExcelHandler {
    /// Write data with an embedded chart to an XLSX file
    pub fn write_with_chart(
        &self,
        path: &str,
        data: &[Vec<String>],
        chart_config: &ChartConfig,
    ) -> Result<()> {
        use super::xlsx_writer::XlsxWriter;
        use super::types::WriteOptions;

        let options = WriteOptions::default();
        let mut writer = XlsxWriter::with_options(options);
        let sheet_name = "Sheet1";
        writer.add_sheet(sheet_name)?;
        writer.add_data(data);
        writer.set_chart(chart_config.clone(), data.to_vec());

        let file = std::fs::File::create(path)?;
        let buf = std::io::BufWriter::new(file);
        writer.save(buf)?;
        Ok(())
    }

    /// Add a chart to existing data and write to an XLSX file
    pub fn add_chart_to_data(
        &self,
        data: &[Vec<String>],
        chart_config: &ChartConfig,
        output_path: &str,
    ) -> Result<()> {
        self.write_with_chart(output_path, data, chart_config)
    }
}
