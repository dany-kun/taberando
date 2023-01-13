use std::error::Error;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn Error>> {
    let file = File::open(".env.json")?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let env_json: serde_json::Value = serde_json::from_reader(reader)?;
    for (key, value) in env_json.as_object().unwrap() {
        println!("cargo:rustc-env={}={}", key, value.as_str().unwrap());
    }
    Ok(())
}
