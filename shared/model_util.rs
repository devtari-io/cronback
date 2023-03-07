use ulid::Ulid;

pub fn generate_model_id<T>(model_prefix: T, owner_id: Option<T>) -> String
where
    T: AsRef<str>,
{
    format!(
        "{}_{}{}",
        model_prefix.as_ref(),
        owner_id
            .map(|f| format!("{}.", f.as_ref()))
            .unwrap_or_default(),
        Ulid::new().to_string()
    )
}

#[test]
fn test_model_id_generation() {
    let id1 = generate_model_id("trig", Some("acc449"));
    assert!(id1.len() > 4);
    assert!(id1.starts_with("trig_acc449."));

    let id1 = generate_model_id("trig", None);
    assert!(id1.len() > 4);
    assert!(!id1.starts_with("trig_acc449"));
}
