use std::fs::File;
use std::io::BufReader;

fn main() {
    let file = match File::open(".env.json") {
        Ok(f) => f,
        Err(_) => {
            return;
        }
    };
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let env_json: serde_json::Value = serde_json::from_reader(reader).unwrap();
    for (key, value) in env_json.as_object().unwrap() {
        println!(
            "cargo:rustc-env={}={}",
            key,
            serde_json::to_string(value).unwrap()
        );
    }
}
