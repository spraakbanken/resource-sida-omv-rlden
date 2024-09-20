#[derive(Debug, Clone)]
pub enum Item {
    Html(String),
    PoddMeta {
        meta_url: String,
        title: String,
        release_date: String,
        description: Option<String>,
        mp3_url: String,
    },
    Podd(Vec<u8>),
}
