mod flags;

fn main() -> anyhow::Result<()> {
    let flags = flags::Bench::from_env_or_exit();
    println!("language: {:?}", flags.language);
    Ok(())
}
