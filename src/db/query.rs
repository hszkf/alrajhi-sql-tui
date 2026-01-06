//! Query execution and result handling

use anyhow::Result;
use chrono::NaiveDateTime;
use std::time::{Duration, Instant};
use tiberius::{Client, Column, ColumnType, Row, numeric::Numeric};
use tokio::net::TcpStream;
use tokio_util::compat::Compat;

/// Represents a cell value in the result set
#[derive(Clone, Debug)]
pub enum CellValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    DateTime(String),
    Binary(Vec<u8>),
}

impl std::fmt::Display for CellValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CellValue::Null => write!(f, "NULL"),
            CellValue::Bool(v) => write!(f, "{}", if *v { "true" } else { "false" }),
            CellValue::Int(v) => write!(f, "{}", v),
            CellValue::Float(v) => write!(f, "{:.6}", v),
            CellValue::String(v) => write!(f, "{}", v),
            CellValue::DateTime(v) => write!(f, "{}", v),
            CellValue::Binary(v) => write!(f, "0x{}", hex::encode(v)),
        }
    }
}

/// Column metadata
#[derive(Clone, Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub max_width: usize,
}

/// Query result
#[derive(Clone, Debug)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<CellValue>>,
    pub row_count: usize,
    pub execution_time: Duration,
    pub affected_rows: Option<u64>,
    pub messages: Vec<String>,
}

impl QueryResult {
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: 0,
            execution_time: Duration::ZERO,
            affected_rows: None,
            messages: Vec::new(),
        }
    }
}

/// Query executor
pub struct QueryExecutor;

impl QueryExecutor {
    /// Execute a query and return results
    pub async fn execute(
        client: &mut Client<Compat<TcpStream>>,
        query: &str,
    ) -> Result<QueryResult> {
        let start = Instant::now();

        // For SELECT * queries, proactively check for DATE columns and rewrite
        let query_to_execute = if Self::is_select_star_query(query) {
            if let Some(fixed_query) = Self::try_fix_date_columns(client, query).await {
                fixed_query
            } else {
                query.to_string()
            }
        } else {
            query.to_string()
        };

        // Execute the query
        let result = client.simple_query(&query_to_execute).await;

        match result {
            Ok(stream) => Self::process_results(stream, start).await,
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("unsupported column type: 40") || err_str.contains("column type: 40") {
                    Err(anyhow::anyhow!(
                        "Table contains DATE columns which are not supported by the driver. \
                        Please cast DATE columns to VARCHAR manually, e.g.:\n\
                        SELECT CONVERT(VARCHAR(10), date_column, 23) as date_column FROM table"
                    ))
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Check if query is a SELECT * query
    fn is_select_star_query(query: &str) -> bool {
        let query_upper = query.to_uppercase();
        let trimmed = query_upper.trim();
        trimmed.starts_with("SELECT") &&
        (trimmed.contains("SELECT *") ||
         (trimmed.contains("SELECT TOP") && trimmed.contains(" * ")))
    }

    /// Process query results from a stream
    async fn process_results(
        stream: tiberius::QueryStream<'_>,
        start: Instant,
    ) -> Result<QueryResult> {
        let mut columns: Vec<ColumnInfo> = Vec::new();
        let mut rows: Vec<Vec<CellValue>> = Vec::new();

        // Process results
        let results = stream.into_results().await?;

        for result in results {
            for row in result {
                if columns.is_empty() {
                    columns = row
                        .columns()
                        .iter()
                        .map(|c| ColumnInfo {
                            name: c.name().to_string(),
                            type_name: format_column_type(c),
                            max_width: c.name().len().max(4),
                        })
                        .collect();
                }

                let mut row_data: Vec<CellValue> = Vec::new();

                for (i, col) in row.columns().iter().enumerate() {
                    let value = extract_cell_value(&row, i, col);
                    let value_len = value.to_string().len();

                    if i < columns.len() {
                        columns[i].max_width = columns[i].max_width.max(value_len);
                    }

                    row_data.push(value);
                }

                rows.push(row_data);
            }
        }

        let execution_time = start.elapsed();

        Ok(QueryResult {
            row_count: rows.len(),
            columns,
            rows,
            execution_time,
            affected_rows: None,
            messages: Vec::new(),
        })
    }

    /// Try to fix a query by casting DATE columns to VARCHAR
    async fn try_fix_date_columns(
        client: &mut Client<Compat<TcpStream>>,
        query: &str,
    ) -> Option<String> {
        let query_upper = query.to_uppercase();

        // Only try to fix SELECT queries
        if !query_upper.trim().starts_with("SELECT") {
            return None;
        }

        // Extract table name from query
        let table_name = Self::extract_table_name(query)?;

        // Get DATE columns for this table
        let date_columns = Self::get_date_columns(client, &table_name).await.ok()?;

        if date_columns.is_empty() {
            return None;
        }

        // Check if query uses SELECT *
        if query_upper.contains("SELECT *") || query_upper.contains("SELECT TOP") && query_upper.contains("*") {
            // Build a new SELECT with proper casting
            return Self::build_select_with_casts(client, query, &table_name, &date_columns).await;
        }

        // For non-SELECT * queries, try simple replacement of column names
        let mut fixed_query = query.to_string();
        for col in &date_columns {
            // Try to wrap existing column references with CONVERT
            let patterns = [
                format!("[{}]", col),
                col.clone(),
            ];
            for pattern in &patterns {
                if fixed_query.contains(pattern) {
                    let replacement = format!("CONVERT(VARCHAR(10), [{}], 23) AS [{}]", col, col);
                    fixed_query = fixed_query.replace(pattern, &replacement);
                    break;
                }
            }
        }

        if fixed_query != query {
            Some(fixed_query)
        } else {
            None
        }
    }

    /// Extract table name from a SELECT query
    fn extract_table_name(query: &str) -> Option<String> {
        let query_upper = query.to_uppercase();

        // Find FROM clause
        let from_pos = query_upper.find(" FROM ")?;
        let after_from = &query[from_pos + 6..];

        // Get the table name (may include schema like dbo.TableName or [dbo].[TableName])
        let table_part: String = after_from
            .trim()
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '(' && *c != ';')
            .collect();

        if table_part.is_empty() {
            None
        } else {
            Some(table_part)
        }
    }

    /// Get DATE columns for a table
    async fn get_date_columns(
        client: &mut Client<Compat<TcpStream>>,
        table_name: &str,
    ) -> Result<Vec<String>> {
        // Parse table name to extract schema and table
        let (schema, table) = Self::parse_table_name(table_name);

        let query = format!(
            "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_NAME = '{}' AND DATA_TYPE = 'date'{}",
            table,
            if let Some(s) = schema {
                format!(" AND TABLE_SCHEMA = '{}'", s)
            } else {
                String::new()
            }
        );

        let stream = client.simple_query(&query).await?;
        let results = stream.into_results().await?;

        let mut date_columns = Vec::new();
        for result in results {
            for row in result {
                if let Some(col_name) = row.get::<&str, _>(0) {
                    date_columns.push(col_name.to_string());
                }
            }
        }

        Ok(date_columns)
    }

    /// Parse table name into schema and table parts
    fn parse_table_name(table_name: &str) -> (Option<String>, String) {
        // Remove brackets and parse
        let clean = table_name.replace(['[', ']'], "");
        let parts: Vec<&str> = clean.split('.').collect();

        match parts.len() {
            1 => (None, parts[0].to_string()),
            2 => (Some(parts[0].to_string()), parts[1].to_string()),
            3 => (Some(parts[1].to_string()), parts[2].to_string()), // database.schema.table
            _ => (None, clean),
        }
    }

    /// Build a SELECT query with proper DATE column casts
    async fn build_select_with_casts(
        client: &mut Client<Compat<TcpStream>>,
        original_query: &str,
        table_name: &str,
        date_columns: &[String],
    ) -> Option<String> {
        // Get all columns for the table
        let (schema, table) = Self::parse_table_name(table_name);

        let query = format!(
            "SELECT COLUMN_NAME, DATA_TYPE FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_NAME = '{}'{}
             ORDER BY ORDINAL_POSITION",
            table,
            if let Some(s) = &schema {
                format!(" AND TABLE_SCHEMA = '{}'", s)
            } else {
                String::new()
            }
        );

        let stream = client.simple_query(&query).await.ok()?;
        let results = stream.into_results().await.ok()?;

        let mut column_defs = Vec::new();
        for result in results {
            for row in result {
                if let (Some(col_name), Some(data_type)) = (
                    row.get::<&str, _>(0),
                    row.get::<&str, _>(1),
                ) {
                    if date_columns.contains(&col_name.to_string()) {
                        // Cast DATE to VARCHAR
                        column_defs.push(format!(
                            "CONVERT(VARCHAR(10), [{}], 23) AS [{}]",
                            col_name, col_name
                        ));
                    } else {
                        column_defs.push(format!("[{}]", col_name));
                    }
                }
            }
        }

        if column_defs.is_empty() {
            return None;
        }

        // Build the new query
        let query_upper = original_query.to_uppercase();

        // Extract TOP clause if present
        let top_clause = if let Some(top_pos) = query_upper.find("TOP ") {
            let after_top = &original_query[top_pos + 4..];
            let top_value: String = after_top
                .trim()
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == ' ')
                .collect();
            format!("TOP {} ", top_value.trim())
        } else {
            String::new()
        };

        // Find WHERE, ORDER BY, etc. to preserve them
        let from_pos = query_upper.find(" FROM ")?;
        let after_from = &original_query[from_pos..];

        let new_query = format!(
            "SELECT {}{}\n{}",
            top_clause,
            column_defs.join(",\n    "),
            after_from.trim()
        );

        Some(new_query)
    }

    /// Execute multiple queries
    pub async fn execute_batch(
        client: &mut Client<Compat<TcpStream>>,
        queries: &[&str],
    ) -> Result<Vec<QueryResult>> {
        let mut results = Vec::new();

        for query in queries {
            let result = Self::execute(client, query).await?;
            results.push(result);
        }

        Ok(results)
    }
}

fn format_column_type(col: &Column) -> String {
    match col.column_type() {
        ColumnType::Null => "NULL".to_string(),
        ColumnType::Bit => "BIT".to_string(),
        ColumnType::Int1 => "TINYINT".to_string(),
        ColumnType::Int2 => "SMALLINT".to_string(),
        ColumnType::Int4 => "INT".to_string(),
        ColumnType::Int8 => "BIGINT".to_string(),
        ColumnType::Float4 => "REAL".to_string(),
        ColumnType::Float8 => "FLOAT".to_string(),
        ColumnType::Datetime => "DATETIME".to_string(),
        ColumnType::Datetime2 => "DATETIME2".to_string(),
        ColumnType::DatetimeOffsetn => "DATETIMEOFFSET".to_string(),
        ColumnType::Daten => "DATE".to_string(),
        ColumnType::Timen => "TIME".to_string(),
        ColumnType::Decimaln => "DECIMAL".to_string(),
        ColumnType::Numericn => "NUMERIC".to_string(),
        ColumnType::Money => "MONEY".to_string(),
        ColumnType::Money4 => "SMALLMONEY".to_string(),
        ColumnType::Guid => "UNIQUEIDENTIFIER".to_string(),
        ColumnType::BigVarChar => "VARCHAR(MAX)".to_string(),
        ColumnType::BigChar => "CHAR".to_string(),
        ColumnType::NVarchar => "NVARCHAR".to_string(),
        ColumnType::NChar => "NCHAR".to_string(),
        ColumnType::Text => "TEXT".to_string(),
        ColumnType::NText => "NTEXT".to_string(),
        ColumnType::BigVarBin => "VARBINARY(MAX)".to_string(),
        ColumnType::BigBinary => "BINARY".to_string(),
        ColumnType::Image => "IMAGE".to_string(),
        ColumnType::Xml => "XML".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

fn extract_cell_value(row: &Row, index: usize, col: &Column) -> CellValue {
    match col.column_type() {
        ColumnType::Null => CellValue::Null,
        ColumnType::Bit => row
            .get::<bool, _>(index)
            .map(CellValue::Bool)
            .unwrap_or(CellValue::Null),
        ColumnType::Int1 => row
            .get::<u8, _>(index)
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnType::Int2 => row
            .get::<i16, _>(index)
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnType::Int4 => row
            .get::<i32, _>(index)
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnType::Int8 => row
            .get::<i64, _>(index)
            .map(CellValue::Int)
            .unwrap_or(CellValue::Null),
        ColumnType::Float4 => row
            .get::<f32, _>(index)
            .map(|v| CellValue::Float(v as f64))
            .unwrap_or(CellValue::Null),
        ColumnType::Float8 => row
            .get::<f64, _>(index)
            .map(CellValue::Float)
            .unwrap_or(CellValue::Null),
        ColumnType::Decimaln | ColumnType::Numericn => row
            .get::<Numeric, _>(index)
            .map(|v| CellValue::String(v.to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::Money | ColumnType::Money4 => row
            .get::<f64, _>(index)
            .map(CellValue::Float)
            .unwrap_or(CellValue::Null),
        ColumnType::Datetime | ColumnType::Datetime2 => row
            .get::<NaiveDateTime, _>(index)
            .map(|v| CellValue::DateTime(v.format("%Y-%m-%d %H:%M:%S").to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::Daten => row
            .get::<NaiveDateTime, _>(index)
            .map(|v| CellValue::DateTime(v.format("%Y-%m-%d").to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::Timen => row
            .get::<NaiveDateTime, _>(index)
            .map(|v| CellValue::DateTime(v.format("%H:%M:%S").to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::BigVarChar
        | ColumnType::BigChar
        | ColumnType::NVarchar
        | ColumnType::NChar
        | ColumnType::Text
        | ColumnType::NText
        | ColumnType::Xml => row
            .get::<&str, _>(index)
            .map(|v| CellValue::String(v.to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::Guid => row
            .get::<tiberius::Uuid, _>(index)
            .map(|v| CellValue::String(v.to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::BigVarBin | ColumnType::BigBinary | ColumnType::Image => row
            .get::<&[u8], _>(index)
            .map(|v| CellValue::Binary(v.to_vec()))
            .unwrap_or(CellValue::Null),
        // Fallback: try various types in order of likelihood
        _ => {
            // Try string first (most common)
            if let Some(v) = row.try_get::<&str, _>(index).ok().flatten() {
                return CellValue::String(v.to_string());
            }
            // Try datetime
            if let Some(v) = row.try_get::<NaiveDateTime, _>(index).ok().flatten() {
                return CellValue::DateTime(v.format("%Y-%m-%d %H:%M:%S").to_string());
            }
            // Try integer
            if let Some(v) = row.try_get::<i64, _>(index).ok().flatten() {
                return CellValue::Int(v);
            }
            // Try float
            if let Some(v) = row.try_get::<f64, _>(index).ok().flatten() {
                return CellValue::Float(v);
            }
            // Try numeric
            if let Some(v) = row.try_get::<Numeric, _>(index).ok().flatten() {
                return CellValue::String(v.to_string());
            }
            // Give up - return type info as string
            CellValue::String(format!("<{:?}>", col.column_type()))
        }
    }
}

// Helper for hex encoding binary data
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02X}", b)).collect()
    }
}
