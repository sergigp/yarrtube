# Summary eval: prompt v3, judge claude-sonnet

Provider anthropic, effort medium, context 131072.

| model | contradicted | critical stated | facts stated | code checks | avg time |
|---|---|---|---|---|---|
| claude-haiku-5-5 | 0/50 | 20/23 | 41/50 | 32/40 | 3s |

## Failed critical facts and contradictions

- **claude-haiku-5-5** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → missing: Summary never mentions the oligarchy's golden-age narrative or the video's rejection of it, only that anarchism flourished due to conservative-order conditions.
- **claude-haiku-5-5** · future-of-capitalism · `decoupling-would-come-too-late` → missing: The summary mentions decoupling as an assumption but says nothing about it taking about 100 years or arriving too late.
- **claude-haiku-5-5** · future-of-capitalism · `conclusion-embrace-degrowth` → missing: It says the transition is 'una disputa entre ideas de futuro', but never says alternative lifestyles must embrace degrowth or that Western consumption cannot be universalized.

## Faithfulness: claims the video doesn't support

| model | major | minor | faithful summaries |
|---|---|---|---|
| claude-haiku-5-5 | 0 | 6 | 6/10 |

- **claude-haiku-5-5** · accelerated-dragon · minor unsupported: "against an opponent who played an open Sicilian structure" → At 0:03-0:11 he only says the opponent may or may not go open Sicilian, and at 16:26 calls the game technical, not aggressive; the opponent's choices (Qa4, Bf4) are not described as an open Sicilian.
- **claude-haiku-5-5** · accelerated-dragon · minor misleading: "though the video never states the final result outright" → He treats the game as won ('easy win' 13:29, 'Kaching' 13:51, 'Keep the pawn' 16:14) and moves on to the next game, so the win is effectively conveyed.
- **claude-haiku-5-5** · accelerated-dragon · minor wrong: "reached a tense king-and-pawn race" → The race at 29:00-33:50 involves rooks and a knight, not just kings and pawns (e.g. 'Rook D1', 'knight C3', 'rook B8').
- **claude-haiku-5-5** · future-of-capitalism · minor misleading: "el comunismo decrecentista y un crecimiento tecnológico selectivo guiado por la necesidad colectiva" → The video presents Saito's degrowth communism as an example of the selective technological degrowth approach, not a separate approach (14:15-14:51).
- **claude-haiku-5-5** · quantum-ai · minor unsupported: "la ventaja cuántica aparece solo en problemas muy artificiales y específicos" → The video says many examples are equally artificial (17:41-17:47) but that problems of great usefulness may exist (17:30-17:40) and quantum data processing is promising (19:26-20:21), so 'solo' overgeneralizes.
- **claude-haiku-5-5** · stonewall-e4-knight · minor wrong: "two games by Karpov and a third game in which White wins" → Only one game is Karpov's (11:41); the second is by 'Hill person'/'Hilap' (18:48, 27:09), and the third is an unattributed extra game (20:28).

## Tokens

- claude-haiku-5-5: 10 calls, 136,475 input + 4,261 output tokens, about $0.016

## Code checks failed

- claude-haiku-5-5 · browser-tabs: direct_start
- claude-haiku-5-5 · build-claude-code: direct_start
- claude-haiku-5-5 · claude-duet-review: length_3_to_5_sentences
- claude-haiku-5-5 · cross-entropy: length_3_to_5_sentences, direct_start
- claude-haiku-5-5 · future-of-capitalism: direct_start
- claude-haiku-5-5 · quantum-ai: direct_start
- claude-haiku-5-5 · stonewall-e4-knight: direct_start
