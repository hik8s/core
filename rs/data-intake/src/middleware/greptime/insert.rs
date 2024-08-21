use greptimedb_ingester::api::v1::{column, Column, ColumnDataType, InsertRequest, SemanticType};
use shared::types::{metadata::Metadata, record::log::LogRecord};

pub fn to_insert_request(logs: &Vec<LogRecord>, metadata: &Metadata) -> InsertRequest {
    let (timestamps, message, record_id) = fold_log_records(logs);

    let columns = vec![
        timestamp_column(timestamps),
        string_column("message", message),
        string_column("record_id", record_id),
    ];

    InsertRequest {
        table_name: metadata.pod_name.to_owned(),
        columns,
        row_count: logs.len() as u32,
    }
}

fn timestamp_column(timestamps: Vec<i64>) -> Column {
    Column {
        column_name: "timestamp".to_owned(),
        values: Some(column::Values {
            timestamp_millisecond_values: timestamps,
            ..Default::default()
        }),
        semantic_type: SemanticType::Timestamp as i32,
        datatype: ColumnDataType::TimestampMillisecond as i32,
        ..Default::default()
    }
}

fn string_column(column_name: &str, data: Vec<String>) -> Column {
    Column {
        column_name: column_name.to_owned(),
        values: Some(column::Values {
            string_values: data,
            ..Default::default()
        }),
        semantic_type: SemanticType::Field as i32,
        datatype: ColumnDataType::String as i32,
        ..Default::default()
    }
}

// Macro to extract parts of logs into separate vectors
macro_rules! fold_records {
    // Define the macro to take an expression ($logs) and a list of identifiers ($field)
    ($logs:expr, $($field:ident),*) => {{
        // Get the number of logs
        let rows = $logs.len();
        ( // Return a tuple containing vectors for each specified field
            $( // Start the repetition for each field provided to the macro
                { // Start a block for each field
                    let mut vec = Vec::with_capacity(rows);
                    for log in $logs {
                        vec.push(log.$field.clone());
                    }
                    vec
                }, // Close the block for each field
            )* // Close the repetition for each field
        ) // Close the tuple that contains all the vectors
    }}; // Closes the macro definition
}

/// Extracts specified fields from a vector of `LogRecord` into separate vectors.
///
/// # Arguments
///
/// * `logs` - A reference to a vector of `LogRecord` to extract fields from.
///
/// # Returns
///
/// A tuple of vectors containing the extracted fields:
/// - timestamps: `Vec<i64>` representing the timestamps.
/// - messages: `Vec<String>` representing the messages.
/// - record_ids: `Vec<String>` representing the record IDs.
fn fold_log_records(logs: &Vec<LogRecord>) -> (Vec<i64>, Vec<String>, Vec<String>) {
    // Use the macro to extract the specified fields from the logs
    fold_records!(logs, timestamp, message, record_id)
}
