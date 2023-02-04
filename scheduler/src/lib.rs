use saffron::Cron;
use chrono::Utc;
use uuid::Uuid;

pub fn start_scheduler() {
    println!("sk_{} !", Uuid::new_v4().as_simple());
    let cron_expr: Cron = "2 4 * * *"
        .parse()
        .expect("Failed to parse cron expression");
    for datetime in cron_expr.iter_after(Utc::now()).take(10) {
        println!("-> {}", datetime);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
