use serde_json::json;

pub fn get_multipart_stream(filename: &str, path: &str, lines: &[String]) -> String {
    let boundary = "boundary";
    let metadata_obj = json!({
        "file": filename,
        "path": path
    });

    let metadata = format!(
        "Content-Disposition: form-data; name=\"metadata\"\r\n\
    Content-Type: application/json\r\n\r\n{}\r\n",
        metadata_obj
    );

    let stream = lines.join("\n");

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
