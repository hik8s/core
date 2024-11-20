use greptimedb_ingester::api::v1::{column, Column, ColumnDataType, InsertRequest, SemanticType};

use crate::types::record::{classified::ClassifiedLogRecord, log::LogRecord};

pub fn logs_to_insert_request(logs: &Vec<LogRecord>, key: &String) -> InsertRequest {
    let (timestamps, message, record_id) = fold_log_records(logs);

    let columns: Vec<Column> = vec![
        timestamp_column(timestamps),
        string_column("message", message),
        tag_column("record_id", record_id),
    ];

    InsertRequest {
        table_name: key.to_owned(),
        columns,
        row_count: logs.len() as u32,
    }
}

pub fn classified_log_to_insert_request(log: ClassifiedLogRecord) -> InsertRequest {
    let key_clone = log.key.to_owned();
    let (
        timestamp,
        message,
        record_id,
        preprocessed_message,
        length,
        class_representation,
        class_id,
        similarity,
        key,
        namespace,
        pod_uid,
        container,
    ) = fold_classified_records(&vec![log]);

    let columns: Vec<Column> = vec![
        timestamp_column(timestamp),
        string_column("message", message),
        tag_column("record_id", record_id),
        string_column("preprocessed_message", preprocessed_message),
        u64_column("length", length),
        string_column("class_representation", class_representation),
        string_column("class_id", class_id),
        f64_column("similarity", similarity),
        string_column("key", key),
        string_column("namespace", namespace),
        string_column("pod_uid", pod_uid),
        string_column("container", container),
    ];

    InsertRequest {
        table_name: key_clone,
        columns,
        row_count: 1,
    }
}

pub fn resource_to_insert_request(
    apiversion: String,
    kind: String,
    name: String,
    uid: String,
    metadata: String,
    namespace: Option<String>,
    spec: Option<String>,
    status: Option<String>,
    timestamp: i64,
) -> InsertRequest {
    let mut columns: Vec<Column> = vec![
        timestamp_column(vec![timestamp]),
        tag_column("uid", vec![uid]),
        string_column("apiversion", vec![apiversion]),
        string_column("name", vec![name]),
        string_column("metadata", vec![metadata]),
    ];
    if namespace.is_some() {
        columns.push(string_column("namespace", vec![namespace.unwrap()]));
    }
    if spec.is_some() {
        columns.push(string_column("spec", vec![spec.unwrap()]));
    }
    if status.is_some() {
        columns.push(string_column("status", vec![status.unwrap()]));
    }

    InsertRequest {
        table_name: kind,
        columns,
        row_count: 1,
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

fn tag_column(column_name: &str, data: Vec<String>) -> Column {
    Column {
        column_name: column_name.to_owned(),
        values: Some(column::Values {
            string_values: data,
            ..Default::default()
        }),
        semantic_type: SemanticType::Tag as i32,
        datatype: ColumnDataType::String as i32,
        ..Default::default()
    }
}

fn u64_column(column_name: &str, data: Vec<u64>) -> Column {
    Column {
        column_name: column_name.to_owned(),
        values: Some(column::Values {
            u64_values: data,
            ..Default::default()
        }),
        semantic_type: SemanticType::Field as i32,
        datatype: ColumnDataType::Uint64 as i32,
        ..Default::default()
    }
}

fn f64_column(column_name: &str, data: Vec<f64>) -> Column {
    Column {
        column_name: column_name.to_owned(),
        values: Some(column::Values {
            f64_values: data,
            ..Default::default()
        }),
        semantic_type: SemanticType::Field as i32,
        datatype: ColumnDataType::Float64 as i32,
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
fn fold_classified_records(
    logs: &Vec<ClassifiedLogRecord>,
) -> (
    Vec<i64>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<u64>,
    Vec<String>,
    Vec<String>,
    Vec<f64>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
) {
    // Use the macro to extract the specified fields from the logs
    fold_records!(
        logs,
        timestamp,
        message,
        record_id,
        preprocessed_message,
        length,
        class_representation,
        class_id,
        similarity,
        key,
        namespace,
        pod_uid,
        container
    )
}
