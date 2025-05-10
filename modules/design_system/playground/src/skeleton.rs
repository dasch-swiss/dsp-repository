#[derive(askama::Template)]
#[template(path = "playground-skeleton.html")]
pub struct PlaygroundSkeleton {
    title: String,
    body: String,
}

impl PlaygroundSkeleton {
    pub fn new(title: String, body: String) -> Self {
        PlaygroundSkeleton { title, body }
    }
}
