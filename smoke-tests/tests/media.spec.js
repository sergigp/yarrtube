import { test } from '../helpers/fixtures.js'
import { expectH264Mp4, expectRangeSupported } from '../helpers/media.js'
import { deleteSeeded, seedDownloadedPlaylist } from '../helpers/seed.js'

test.use({ keepLibrary: true })

let seeded
test.beforeAll(async ({ request }) => {
  seeded = await seedDownloadedPlaylist(request)
})

test.afterAll(async ({ request }) => {
  await deleteSeeded(request, seeded)
})

test('downloaded video is h264/aac mp4', async () => {
  await expectH264Mp4(`${seeded.path}/${seeded.video.filename}`)
})

test.fixme('media answers a Range request with 206', async () => {})
