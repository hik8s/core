use greptimedb_ingester::api::v1::{column, Column, ColumnDataType, InsertRequest, SemanticType};
use shared::types::{metadata::Metadata, parsedline::ParsedLine};

pub fn to_insert_request(logs: &Vec<ParsedLine>, metadata: &Metadata) -> InsertRequest {
    let rows = logs.len();

    let (ts, text, id) = logs.into_iter().fold(
        (
            Vec::with_capacity(rows),
            Vec::with_capacity(rows),
            Vec::with_capacity(rows),
        ),
        |mut acc, log| {
            acc.0.push(log.timestamp);
            acc.1.push(log.text.clone());
            acc.2.push(log.id.clone());
            acc
        },
    );

    let columns = vec![
        // timestamp column: ts
        Column {
            column_name: "ts".to_owned(),
            values: Some(column::Values {
                timestamp_millisecond_values: ts,
                ..Default::default()
            }),
            semantic_type: SemanticType::Timestamp as i32,
            datatype: ColumnDataType::TimestampMillisecond as i32,
            ..Default::default()
        },
        // field column: text
        Column {
            column_name: "text".to_owned(),
            values: Some(column::Values {
                string_values: text.into_iter().collect(),
                ..Default::default()
            }),
            semantic_type: SemanticType::Field as i32,
            datatype: ColumnDataType::String as i32,
            ..Default::default()
        },
        // id column
        Column {
            column_name: "id".to_owned(),
            values: Some(column::Values {
                string_values: id,
                ..Default::default()
            }),
            semantic_type: SemanticType::Field as i32,
            datatype: ColumnDataType::String as i32,
            ..Default::default()
        },
    ];

    InsertRequest {
        table_name: metadata.pod_name.to_owned(),
        columns,
        row_count: rows as u32,
    }
}
