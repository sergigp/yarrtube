## MODIFIED Requirements

### Requirement: Add Channel Destination Notice
The add channel dialog SHALL look the entered channel up on YouTube once the channel handle or URL field stops changing, and SHALL show a notice between that field and the "Advanced options" section. No notice SHALL be shown while the field is empty. While the lookup is in progress the notice SHALL say that the channel is being looked up. A lookup answered for a value the field no longer holds SHALL be ignored.

Once the channel has been found, the notice SHALL show the channel's avatar, when YouTube reports one, and state how many of the channel's latest videos will be downloaded (the video limit), the channel's YouTube title, and the absolute destination they will be downloaded to (the videos root, parent folder and folder name joined together). It SHALL update as the video limit, parent folder or folder name changes, and SHALL offer a "change" action that expands "Advanced options". The folder name continues to derive from the entered handle, not from the title.

The notice SHALL instead be shown as an error, and the dialog SHALL NOT submit the request, when:

- the value is not a channel handle or channel URL, or the channel does not exist or is not accessible on YouTube, stating which;
- YouTube could not be reached, saying so;
- the channel is already tracked, stating that it has already been added and the name it was added under;
- the destination is already the storage location of another playlist or channel, naming the destination and the playlist or channel occupying it, and still offering the "change" action.

When more than one applies, the first in this list SHALL be shown. The dialog SHALL NOT submit the request until the channel has been found.

The add channel dialog SHALL NOT show a separate destination preview or report which directories will be created or already exist.

#### Scenario: Notice hidden before a handle is entered
- **WHEN** a user opens the add channel dialog and the channel handle or URL field is empty
- **THEN** no notice is shown and the dialog does not submit

#### Scenario: Notice states the video limit and destination
- **WHEN** a user enters `@veritasium`, the handle of a channel titled "Veritasium", without changing any advanced option
- **THEN** once looked up, the notice shows the channel's avatar and states that the latest 3 videos from "Veritasium" will be downloaded to the videos root joined with `channels/veritasium`

#### Scenario: Channel without an avatar
- **WHEN** the entered channel has no avatar on YouTube
- **THEN** the notice shows a placeholder in the avatar's place and otherwise reads the same

#### Scenario: Notice follows the advanced options
- **WHEN** a user changes the video limit, the parent folder or the folder name
- **THEN** the notice's video count and destination update to match

#### Scenario: Change action expands advanced options
- **WHEN** a user activates the notice's "change" action
- **THEN** "Advanced options" expands, showing the storage location controls

#### Scenario: Value that is not a channel
- **WHEN** a user enters a value that is not a handle or channel URL, such as a handle without its leading `@`
- **THEN** the notice is shown as an error explaining why the value is not a channel, and the dialog does not submit the request

#### Scenario: Channel not found
- **WHEN** a user enters the handle of a channel that does not exist or is not accessible
- **THEN** the notice is shown as an error stating that the channel was not found, and the dialog does not submit the request

#### Scenario: Channel already added
- **WHEN** a user enters the handle or URL of a channel that is already tracked under the name "Veritasium"
- **THEN** the notice is shown as an error stating that it was already added as "Veritasium", and the dialog does not submit the request

#### Scenario: Destination already in use
- **WHEN** the composed destination is the storage location of another playlist or channel
- **THEN** the notice is shown as an error naming the destination and the playlist or channel occupying it, and the dialog does not submit the request
