use ulid::Ulid;

pub fn generate_model_id<T>(model_prefix: T) -> String
where
    T: AsRef<str>,
{
    format!("{}_{}", model_prefix.as_ref(), Ulid::new().to_string())
}

#[test]
fn test_model_id_generation() {
    let id1 = generate_model_id("trig");
    assert!(id1.len() > 4);
    assert!(id1.starts_with("trig_"));
}
