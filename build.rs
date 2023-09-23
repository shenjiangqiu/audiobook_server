use chrono::{DateTime, Utc};

fn main() {
    let now: DateTime<Utc> = Utc::now();
    // let offset = FixedOffset::east_opt(0).unwrap(); // GMT
    // let now = now.with_timezone(&offset);
    // let items = StrftimeItems::new("%a, %d %b %Y %H:%M:%S GMT"); // Define the timestamp format
    // let formatted_date = now.format_with_items(items).to_string();
    // set env BUILD_TIME
    println!("cargo:rustc-env=BUILD_TIME={}", now.timestamp());
}
