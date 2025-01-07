use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};

fn clean_json_string(s: &str) -> String {
    s.trim_matches('"').to_string()
}
// save_resource(&data.data, "resource", ".testdata8").unwrap();
pub fn save_resource(
    resource_json: &serde_json::Value,
    route: &str,
    dir: &str,
) -> io::Result<PathBuf> {
    let name = resource_json
        .get("metadata")
        .and_then(|m| m.get("name"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing name"))?
        .as_str()
        .map(clean_json_string)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Name is not a string"))?;

    let uid = resource_json
        .get("metadata")
        .and_then(|m| m.get("uid"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing uid"))?
        .as_str()
        .map(clean_json_string)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "uid is not a string"))?;

    let resource_version = resource_json
        .get("metadata")
        .and_then(|m| m.get("resourceVersion"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing uid"))?
        .as_str()
        .map(clean_json_string)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "uid is not a string"))?;

    let namspace_unset = "not-namespaced".to_string();
    let namespace = resource_json
        .get("metadata")
        .and_then(|m| m.get("namespace"))
        .and_then(|n| n.as_str())
        .map(clean_json_string)
        .unwrap_or(namspace_unset);

    let kind = resource_json
        .get("kind")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing kind"))?
        .as_str()
        .map(clean_json_string)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Kind is not a string"))?;

    let path = Path::new(dir).join(route);
    fs::create_dir_all(&path)?;

    let file_name = format!(
        "{}_{}_{}_{}_{}.yaml",
        kind.to_lowercase(),
        namespace,
        name,
        uid,
        resource_version,
    );
    let file_path = path.join(file_name);

    let yaml_string = serde_yaml::to_string(&resource_json)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut file = File::create(&file_path)?;
    file.write_all(yaml_string.as_bytes())?;

    Ok(file_path)
}
