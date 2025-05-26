mod indexer;

fn main() {
    let index = indexer::index(
        String::from("/hdd/media/Movies"),
        String::from("/hdd/media/TV"),
    );
}
