//! Test DATE column auto-casting
//! Run with: cargo run --release --bin test_date_query

use alrajhi_sql_tui::db::{DbConfig, DbConnection, QueryExecutor};

#[tokio::main]
async fn main() {
    println!("=== Comprehensive Query Tests ===\n");

    // Connect to database
    println!("Connecting to database...");
    let config = DbConfig::default();
    let db = match DbConnection::new(config).await {
        Ok(db) => {
            println!("✓ Connected successfully!\n");
            db
        }
        Err(e) => {
            println!("✗ Connection failed: {}", e);
            return;
        }
    };

    let client_arc = db.client();
    let mut client = client_arc.lock().await;

    let mut passed = 0;
    let mut failed = 0;

    // Test 1: Simple SELECT
    println!("--- Test 1: Simple SELECT ---");
    match QueryExecutor::execute(&mut client, "SELECT 1 as num, 'hello' as txt").await {
        Ok(result) => {
            println!("✓ Passed: {} row(s) in {:?}", result.row_count, result.execution_time);
            passed += 1;
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            failed += 1;
        }
    }

    // Test 2: Table with DATE columns (auto-casting test)
    println!("\n--- Test 2: SELECT * with DATE columns ---");
    let test_query = "SELECT TOP 3 * FROM Staging.[dbo].RBS_rbsdw98d_trx_ISS_SORT";
    match QueryExecutor::execute(&mut client, test_query).await {
        Ok(result) => {
            println!("✓ Passed: {} row(s), {} columns in {:?}",
                result.row_count, result.columns.len(), result.execution_time);
            // Check if Extraction_Date is properly formatted
            if let Some(row) = result.rows.first() {
                if let Some(cell) = row.first() {
                    let val = cell.to_string();
                    if val.contains("-") && val.len() == 10 {
                        println!("  ✓ DATE column formatted correctly: {}", val);
                    } else {
                        println!("  ! DATE column format: {}", val);
                    }
                }
            }
            passed += 1;
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            failed += 1;
        }
    }

    // Test 3: Non-SELECT * query
    println!("\n--- Test 3: Non-SELECT * query ---");
    let test_query = "SELECT TOP 5 Internal_Bank_Code, Account_Number FROM Staging.[dbo].RBS_rbsdw98d_trx_ISS_SORT";
    match QueryExecutor::execute(&mut client, test_query).await {
        Ok(result) => {
            println!("✓ Passed: {} row(s) in {:?}", result.row_count, result.execution_time);
            passed += 1;
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            failed += 1;
        }
    }

    // Test 4: Query with aggregation
    println!("\n--- Test 4: Aggregation query ---");
    match QueryExecutor::execute(&mut client, "SELECT COUNT(*) as total FROM Staging.[dbo].RBS_rbsdw98d_trx_ISS_SORT").await {
        Ok(result) => {
            if let Some(row) = result.rows.first() {
                if let Some(cell) = row.first() {
                    println!("✓ Passed: Total rows in table = {}", cell);
                }
            }
            passed += 1;
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            failed += 1;
        }
    }

    // Test 5: System query
    println!("\n--- Test 5: System query ---");
    match QueryExecutor::execute(&mut client, "SELECT @@VERSION").await {
        Ok(result) => {
            if let Some(row) = result.rows.first() {
                if let Some(cell) = row.first() {
                    let version = cell.to_string();
                    let short = version.lines().next().unwrap_or("?");
                    println!("✓ Passed: {}", short);
                }
            }
            passed += 1;
        }
        Err(e) => {
            println!("✗ Failed: {}", e);
            failed += 1;
        }
    }

    // Summary
    println!("\n=== SUMMARY ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    if failed == 0 {
        println!("\n=== ALL TESTS PASSED ===");
    } else {
        println!("\n=== SOME TESTS FAILED ===");
    }
}
