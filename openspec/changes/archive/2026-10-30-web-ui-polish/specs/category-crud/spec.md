## MODIFIED Requirements

### Requirement: Category Color
Every category SHALL have a color: a `#` followed by 6 hexadecimal digits, stored and returned lowercase. Any other value SHALL be rejected with HTTP status 400 and nothing recorded. The palette SHALL be, in order: `#9f8401`, `#bd7123`, `#c8645b`, `#bd6396`, `#9870c5`, `#6a80d4`, `#0193bf`, `#019b8d`, `#79922c`, `#978364`. Any valid color SHALL be accepted, whether or not it is in the palette.

#### Scenario: Color normalised to lowercase
- **WHEN** a category is created with the color "#0193BF"
- **THEN** its color is "#0193bf"

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
- **THEN** its color is "#9f8401"

#### Scenario: Next unused color
- **WHEN** a category is created without a color and the existing categories use "#9f8401" and "#c8645b"
- **THEN** its color is "#bd7123"

#### Scenario: Every palette color used
- **WHEN** a category is created without a color, every palette color is used once and "#9f8401" is used twice
- **THEN** its color is "#bd7123"

### Requirement: Existing Categories Get Colors
On upgrade, every category that existed before colors were introduced SHALL be given a palette color, assigned in palette order by creation time and wrapping after the tenth.

#### Scenario: Upgrading with categories
- **WHEN** the daemon upgrades a database holding uncolored categories "Chess", "Science" and "Sports", created in that order
- **THEN** their colors are "#9f8401", "#bd7123" and "#c8645b"

## ADDED Requirements

### Requirement: Pastel Colors Strengthened On Upgrade
On upgrade, every category whose color is one of the former pastel palette colors SHALL get the palette color at the same position: `#fff1bf` → `#9f8401`, `#ffe0c7` → `#bd7123`, `#ffd9d4` → `#c8645b`, `#fadceb` → `#bd6396`, `#e9def8` → `#9870c5`, `#dbe3fb` → `#6a80d4`, `#d3ecf8` → `#0193bf`, `#cff0ea` → `#019b8d`, `#e4f0cc` → `#79922c`, `#eee5d7` → `#978364`. Every other color that is light, with an OKLCH lightness above 0.85, SHALL get the color of the same hue at the palette's solid lightness (0.62) and chroma (0.13, lowered as needed to fit sRGB). Any other color SHALL be left unchanged. The upgrade SHALL run once.

#### Scenario: Former palette color
- **WHEN** the daemon upgrades a database holding the category "Chess" colored "#d3ecf8"
- **THEN** "Chess" is colored "#0193bf"

#### Scenario: Former custom pastel
- **WHEN** the daemon upgrades a database holding a category colored with a custom pastel of hue 150
- **THEN** that category's color has hue 150 at the palette's solid lightness and chroma

#### Scenario: Already strong color
- **WHEN** the daemon upgrades a database holding a category colored "#c8645b"
- **THEN** its color stays "#c8645b"
