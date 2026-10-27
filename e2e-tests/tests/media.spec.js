import { test } from '../helpers/fixtures.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import { expectRangeSupported } from '../helpers/media.js'
import { addPlaylist } from '../helpers/seed.js'

// That real downloads are H.264/AAC `.mp4` is checked by the YouTube
// contract tests; here the file is the fake YouTube's sample.
test('media answers a Range request with 206', async ({ request }) => {
  const playlist = await addPlaylist(request, PLAYLIST)
  const [video] = playlist.videos

  // The URL the player loads (see `videoMediaUrl` in the web UI).
  const encode = (path) => path.split('/').map(encodeURIComponent).join('/')
  await expectRangeSupported(request, `/media/${encode(playlist.path)}/${encode(video.filename)}`)
})
