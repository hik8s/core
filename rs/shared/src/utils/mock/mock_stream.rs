use serde_json::json;

use super::mock_data::TestData;

pub fn get_multipart_stream(test_data: &TestData) -> String {
    let boundary = "boundary";
    let metadata_obj = json!({
        "file": test_data.metadata.filename,
        "path": test_data.metadata.path,
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
