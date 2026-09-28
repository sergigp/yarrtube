import { videoMediaUrl } from '../api'

/**
 * The player area of a playlist or channel detail view: plays `video` from
 * `basePath` once downloaded, or explains why it can't yet.
 */
export function VideoPlayer({ basePath, video, autoplay, onVideoElement }) {
  return (
    <div className="flex min-h-80 items-center justify-center rounded-lg bg-secondary/60">
      {video?.status === 'DOWNLOADED' && video.filename ? (
        // eslint-disable-next-line jsx-a11y/media-has-caption
        <video
          ref={onVideoElement}
          controls
          autoPlay={autoplay}
          className="block max-h-[70vh] w-full rounded-lg"
          src={videoMediaUrl(basePath, video.filename)}
          poster={
            video.thumbnail_filename ? videoMediaUrl(basePath, video.thumbnail_filename) : undefined
          }
        />
      ) : (
        <p className="text-sm text-muted-foreground">
          {video ? 'This video has not been downloaded yet.' : 'Select a video to play it.'}
        </p>
      )}
    </div>
  )
}
