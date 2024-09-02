use shared::types::class::vectorized::VectorizedClass;

pub fn create_augmented_prompt(
    user_message: &str,
    kube_system: Vec<VectorizedClass>,
    other: Vec<VectorizedClass>,
) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!(
        "## The user has asked this question: \"{user_message}\"\n\n"
    ));
    prompt.push_str("To answer this question, consider the following logs:\n\n");
    prompt.push_str(
        "### The following logs are given in the format <namespace>/<pod>, score <score>: <log_message>. In the log_message you will find expressions <var>. These are variable data that can be uuids, names, timestamps or something else.\n",
    );
    for vc in other.into_iter() {
        prompt.push_str(&format_log_entry(&vc));
    }
    prompt.push_str("\n### We have also found the following logs in the kube-system namespace. The format is <pod>, score: <log_message>\n");
    for vc in kube_system.into_iter() {
        prompt.push_str(&format_log_entry(&vc));
    }
    prompt
}

fn format_log_entry(vc: &VectorizedClass) -> String {
    format!(
        "{}/{}, Score {}: {}\n",
        vc.namespace, vc.key, vc.score, vc.representation
    )
}
