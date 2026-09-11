use rust_embed::RustEmbed;

/// The built single-page application (`web/dist/`, produced by `npm run
/// build`), baked into the binary at compile time so the daemon can serve it
/// without any file present at runtime.
#[derive(RustEmbed)]
#[folder = "web/dist/"]
pub struct WebAssets;
