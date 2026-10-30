# Spec Delta

## MODIFIED Requirements

### Requirement: Category Color
Every category SHALL have a color: a `#` followed by 6 hexadecimal digits, stored and returned lowercase. Any other value SHALL be rejected with HTTP status 400 and nothing recorded. The palette SHALL be, in order: `#f59e0b`, `#f97316`, `#ef4444`, `#ec4899`, `#8b5cf6`, `#3b82f6`, `#0ea5e9`, `#14b8a6`, `#22c55e`, `#64748b`. Any valid color SHALL be accepted, whether or not it is in the palette.

#### Scenario: Color normalised to lowercase
- **WHEN** a category is created with the color "#0EA5E9"
- **THEN** its color is "#0ea5e9"

#### Scenario: Custom color
- **WHEN** a category is created with the color "#f0e4d0", which is not in the palette
- **THEN** its color is "#f0e4d0"

#### Scenario: Invalid color
- **WHEN** a category is created or updated with the color "yellow", "#fff" or "#gggggg"
- **THEN** the daemon responds with HTTP status 400 and records nothing

### Requirement: Default Category Color
A category created without a color SHALL get the palette color used by the fewest categories, the earliest in palette order on a tie.

#### Scenario: First category
- **WHEN** a category is created without a color and no category exists
- **THEN** its color is "#f59e0b"

#### Scenario: Next unused color
- **WHEN** a category is created without a color and the existing categories use "#f59e0b" and "#ef4444"
- **THEN** its color is "#f97316"

#### Scenario: Every palette color used
- **WHEN** a category is created without a color, every palette color is used once and "#f59e0b" is used twice
- **THEN** its color is "#f97316"

### Requirement: Existing Categories Get Colors
On upgrade, every category that existed before colors were introduced SHALL be given a palette color, assigned in palette order by creation time and wrapping after the tenth.

#### Scenario: Upgrading with categories
- **WHEN** the daemon upgrades a database holding uncolored categories "Chess", "Science" and "Sports", created in that order
- **THEN** their colors are "#f59e0b", "#f97316" and "#ef4444"

### Requirement: Pastel Colors Strengthened On Upgrade
On upgrade, every category whose color is one of the former pastel palette colors SHALL get the palette color at the same position: `#fff1bf` → `#f59e0b`, `#ffe0c7` → `#f97316`, `#ffd9d4` → `#ef4444`, `#fadceb` → `#ec4899`, `#e9def8` → `#8b5cf6`, `#dbe3fb` → `#3b82f6`, `#d3ecf8` → `#0ea5e9`, `#cff0ea` → `#14b8a6`, `#e4f0cc` → `#22c55e`, `#eee5d7` → `#64748b`. Every other color that is light, with an OKLCH lightness above 0.85, SHALL get the color of the same hue at OKLCH lightness 0.62 and chroma 0.13 (lowered as needed to fit sRGB). Any other color SHALL be left unchanged. The upgrade SHALL run once.

#### Scenario: Former palette color
- **WHEN** the daemon upgrades a database holding the category "Chess" colored "#d3ecf8"
- **THEN** "Chess" is colored "#0ea5e9"

#### Scenario: Former custom pastel
- **WHEN** the daemon upgrades a database holding a category colored with a custom pastel of hue 150
- **THEN** that category's color has hue 150 at OKLCH lightness 0.62 and chroma 0.13

#### Scenario: Already strong color
- **WHEN** the daemon upgrades a database holding a category colored "#03999f"
- **THEN** its color stays "#03999f"

## ADDED Requirements

### Requirement: Former Palette Colors Refreshed On Upgrade
On upgrade, every category whose color is one of the former solid palette colors SHALL get the palette color at the same position: `#9f8401` → `#f59e0b`, `#bd7123` → `#f97316`, `#c8645b` → `#ef4444`, `#bd6396` → `#ec4899`, `#9870c5` → `#8b5cf6`, `#6a80d4` → `#3b82f6`, `#0193bf` → `#0ea5e9`, `#019b8d` → `#14b8a6`, `#79922c` → `#22c55e`, `#978364` → `#64748b`. Any other color SHALL be left unchanged. The upgrade SHALL run once.

#### Scenario: Former palette color
- **WHEN** the daemon upgrades a database holding the category "Chess" colored "#0193bf"
- **THEN** "Chess" is colored "#0ea5e9"

#### Scenario: Custom color kept
- **WHEN** the daemon upgrades a database holding a category colored "#03999f", which is in neither palette
- **THEN** that category is still colored "#03999f"
