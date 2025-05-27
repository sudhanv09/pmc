mod indexer;
mod mpv;
mod library;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let index = indexer::index(
        String::from("/hdd/media/Movies"),
        String::from("/hdd/media/TV"),
    );

    let mut mpv = mpv::Player::init("/tmp/mpvipc").await?;

    println!("{:?}", &index.movies[0].name);
    mpv.play_file(&index.movies[0].path).await?;

    Ok(())
}
