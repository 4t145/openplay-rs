use openplay_client::default_user_dir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_dir = default_user_dir()?;
    println!("{}", user_dir.to_string_lossy());
    Ok(())
}
