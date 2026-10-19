import { slugOf } from '../helpers/addDialog.js'

// What `seed.spec.js` leaves behind on the released image and
// `verify.spec.js` expects this build to carry on.
export const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
export const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'test'
// The API takes no default path; this is the one the Add dialog derives.
export const PLAYLIST_PATH = `playlists/${slugOf(PLAYLIST_NAME)}`
// Past the 30 seconds that count a video as started, well short of watched.
export const POSITION_SECONDS = 120
// Not the 3-hour default, so a setting that was dropped shows.
export const RECONCILE_INTERVAL_MINUTES = 45
