use ulid::Ulid;

pub(crate) fn generate_model_id<T>(model_prefix: T, owner_id: T) -> String
where
    T: AsRef<str>,
{
    format!(
        "{}_{}.{}",
        model_prefix.as_ref(),
        owner_id.as_ref(),
        Ulid::new().to_string()
    )
}

pub(crate) fn generate_owner_id<T>(model_prefix: T) -> String
where
    T: AsRef<str>,
{
    format!("{}_{}", model_prefix.as_ref(), Ulid::new().to_string())
}

#[test]
fn test_model_id_generation() {
    let id1 = generate_model_id("trig", "acc449");
    assert!(id1.len() > 4);
    assert!(id1.starts_with("trig_acc449."));

    let id1 = generate_owner_id("acc");
    assert!(id1.len() > 4);
    assert!(id1.starts_with("acc_"));
}
