# Spec Delta

## ADDED Requirements

### Requirement: Preferences Saved One At A Time
While a preference is being saved, every switch on the preferences view SHALL be disabled, so a second change is always based on the saved first one.

#### Scenario: Toggling while a save is in flight
- **WHEN** a user turns off "Show quick watches" and its save has not finished yet
- **THEN** every preference switch is disabled until it finishes

#### Scenario: Two changes in a row
- **WHEN** a user turns off "Show quick watches" and, once saved, turns off "Show latest videos"
- **THEN** both are saved off

### Requirement: Home Reflects A Preferences Change At Once
After a preference is saved, the home view SHALL NOT show sections from before the change, even when it was showing them earlier. While categories are off, the home view SHALL NOT show category sections, whatever home response it holds.

#### Scenario: Back to home after turning categories off
- **WHEN** home showed "More about Chess", and the user turns off "Enable categories" and goes back to home
- **THEN** home shows no "More about Chess" section, not even briefly

#### Scenario: Home videos reloaded on save
- **WHEN** a user saves a preference while home's videos are cached but home is not shown
- **THEN** the home videos are reloaded straight away
