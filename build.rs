use std::time::SystemTime;

fn main() {
	// Get current UTC timestamp
	let now = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap();
	
	// Convert timestamp to string
	let timestamp = now.as_secs().to_string();
	
	// Set the build date as an environment variable for compile time
	println!("cargo:rustc-env=BUILD_DATE={}", timestamp);
}
