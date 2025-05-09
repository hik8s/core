use shared::{
    constant::DEFAULT_NAME,
    utils::{get_as_option_string, get_as_string},
};
use uuid7::uuid4;

pub fn extract_owner_aggregated_value(
    metadata: &serde_json::Value,
    key: &str,
    field: &str,
) -> Option<String> {
    metadata
        .get(key)
        .and_then(|owner_references| {
            owner_references.as_array().map(|refs| {
                refs.iter()
                    .filter_map(|owner| get_as_option_string(owner, field))
                    .collect::<Vec<String>>()
            })
        })
        .map(|values| values.join("_"))
        .filter(|s| !s.is_empty())
}

pub fn extract_name_and_owner_name(metadata: &serde_json::Value) -> (String, String) {
    let name = get_as_string(metadata, "name").unwrap_or(DEFAULT_NAME.to_string());
    let owner_name =
        extract_owner_aggregated_value(metadata, "ownerReferences", "name").unwrap_or(name.clone());

    (name, owner_name)
}

use backtrace::Backtrace;
pub fn marked_uid() -> String {
    let uuid = uuid4().to_string();
    let backtrace = Backtrace::new();
    let marked_uid = format!("00000000{}", &uuid[8..]);
    tracing::debug!("Generated marked UID: {marked_uid} - Called from:\n{backtrace:?}");
    marked_uid
}

pub fn extract_uid_and_owner_uid(metadata: &serde_json::Value) -> (String, String) {
    let uid = get_as_string(metadata, "uid")
        .inspect_err(|e| tracing::error!("Error parsing uid: {e}"))
        .unwrap_or(marked_uid());
    let owner_uid =
        extract_owner_aggregated_value(metadata, "ownerReferences", "uid").unwrap_or(uid.clone());

    (uid, owner_uid)
}

#[cfg(test)]
mod tests {
    use shared::setup_tracing;

    use super::*;

    #[test]
    fn test_uuid_zero_prefix() {
        // Generate a sample UUID
        setup_tracing(false);
        let test_uuid = uuid4().to_string();

        // Verify the length is 36 characters (standard UUID format)
        assert_eq!(test_uuid.len(), 36);

        // Create modified UUID with zeros
        let modified = marked_uid();
        tracing::debug!("Modified UUID: {}", modified);
        // Check that the modified UUID starts with 8 zeros
        assert_eq!(&modified[..8], "00000000");

        // Check that the overall length is still correct
        assert_eq!(modified.len(), 36);
    }
}
