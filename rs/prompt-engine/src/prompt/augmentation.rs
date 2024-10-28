use shared::types::class::vectorized::VectorizedClass;

use super::understanding::UnderstandingOutput;

pub fn create_augmented_prompt(
    user_message: &str,
    kube_system: Vec<VectorizedClass>,
    other: Vec<VectorizedClass>,
    understanding: &UnderstandingOutput,
) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!(
        "The user has asked this question: '{user_message}'."
    ));
    if let Some(intention) = &understanding.intention {
        prompt.push_str(&format!("\nThe user's intention could be: {intention}"));
    }

    prompt.push_str(
        format!(
            "\nThese may be relevant keywords: {}",
            understanding.keywords.join(",")
        )
        .as_str(),
    );
    prompt.push_str("\nTo answer this question, consider the following logs - they have the format <namespace>/<pod>, <score>: <log_message>. In the log_message you will find expressions '<var>', for variable data like uuid etc.");

    for vc in other.into_iter() {
        prompt.push_str(&format_log_entry(&vc));
    }
    prompt.push_str("\nWe have also found the following logs in the kube-system namespace:");
    for vc in kube_system.into_iter() {
        prompt.push_str(&format_log_entry(&vc));
    }
    prompt
}

fn format_log_entry(vc: &VectorizedClass) -> String {
    format!(
        "\n{}/{}, Score {}: {}",
        vc.namespace, vc.key, vc.score, vc.representation
    )
}
