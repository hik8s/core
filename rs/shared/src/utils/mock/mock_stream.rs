use serde_json::json;

use crate::utils::mock::mock_client::{generate_random_filename, get_test_path};

use super::mock_data::TestData;

pub fn get_multipart_stream(test_data: &TestData) -> String {
    let boundary = "boundary";
    let metadata_obj = json!({
        "file": generate_random_filename(),
        "path": get_test_path(&test_data.metadata.pod_name),
    });

    let metadata = format!(
        "Content-Disposition: form-data; name=\"metadata\"\r\n\
    Content-Type: application/json\r\n\r\n{}\r\n",
        metadata_obj
    );

    let stream = test_data.raw_messages.join("\n");

    let stream = format!(
        "Content-Disposition: form-data; name=\"stream\"\r\n\
    Content-Type: application/octet-stream\r\n\r\n{}\n",
        stream
    );

    format!(
        "--{}\r\n{}\r\n--{}\r\n{}\r\n--{}--\r\n",
        boundary, metadata, boundary, stream, boundary
    )
}
