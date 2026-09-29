import { DestinationNotice } from './DestinationNotice'

/**
 * The add dialogs' notice while a `useLookup` lookup hasn't found something
 * addable: it is still running, it failed, or what it found is already
 * tracked. Each of these blocks submission, so it outranks the destination;
 * null once none applies. `noun` names what is looked up ("playlist").
 */
export function lookupNotice(noun, { lookingUp, preview, tracked }) {
  if (lookingUp) {
    return <DestinationNotice tone="info">{`Looking up ${noun}…`}</DestinationNotice>
  }
  if (preview.error) {
    return <DestinationNotice tone="error">{preview.error.message}</DestinationNotice>
  }
  if (tracked) {
    return <DestinationNotice tone="error">Already added as “{tracked.name}”</DestinationNotice>
  }
  return null
}
