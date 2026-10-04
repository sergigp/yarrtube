/// A metadata match Plex offers for a library item ("Fix Match"), e.g. the
/// NFO agent's `tv.plex.agents.nfo.movie://movie/youtube_<id>` built from
/// the item's `movie.nfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexMatchCandidate {
    pub guid: String,
    pub name: String,
}
