use anyhow::Result;
use zsh_history_to_fish::Converter;

#[tokio::main]
async fn main() -> Result<()> {
    Converter::from_args()
        .await?
        .convert()
        .await?
        .iter()
        .for_each(|entry| println!("{entry}"));

    Ok(())
}
