use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub filename: String,
    pub path: String,
    pub namespace: String,
    pub pod_name: String,
    pub pod_uid: String,
    pub container: String,
}

impl Metadata {
    pub fn from_path(filename: &str, path: &str) -> Result<Self, Box<dyn Error>> {
        // Deconstruct namespace, pod and container
        let components: Vec<&str> = path.split('/').collect();

        // Check if components has at least 6 elements
        if components.len() < 6 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Invalid path: {} | expected /var/log/pods/<pod>/<container>",
                    path
                ),
            )));
        }

        // Extract the namespace, pod_name, and pod_uid
        let ns_pod_id: Vec<&str> = components[4].split('_').collect();

        // Check if ns_pod_id has exactly 3 elements
        if ns_pod_id.len() != 3 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Invalid format for namespace, pod name and pod uid: {}",
                    components[4]
                ),
            )));
        }

        let namespace = ns_pod_id[0].to_string();
        let pod_name = ns_pod_id[1].to_string();
        let pod_uid = ns_pod_id[2].to_string();

        // Extract the container
        let container = components[5].to_string();

        // Initialize Metadata struct
        Ok(Self {
            filename: filename.to_string(),
            path: path.to_string(),
            namespace,
            pod_name,
            pod_uid,
            container,
        })
    }
    pub fn test_path(podname: &str) -> String {
        format!(
            "/var/log/pods/test-ns_{}_123-4123-53754/some-container",
            podname
        )
    }
}
