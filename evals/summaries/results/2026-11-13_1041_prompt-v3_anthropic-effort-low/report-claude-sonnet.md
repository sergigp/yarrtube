# Summary eval: prompt v3, judge claude-sonnet

Provider anthropic, effort low, context 131072.

| model | contradicted | critical stated | facts stated | code checks | avg time |
|---|---|---|---|---|---|
| claude-haiku-5-5 | 0/50 | 22/23 | 46/50 | 32/40 | 2s |

## Failed critical facts and contradictions

- **claude-haiku-5-5** · future-of-capitalism · `decoupling-would-come-too-late` → missing: Summary only says decoupling is "empíricamente discutido"; it never mentions the 100-year timescale versus 10-20 years.

## Faithfulness: claims the video doesn't support

| model | major | minor | faithful summaries |
|---|---|---|---|
| claude-haiku-5-5 | 0 | 6 | 6/10 |

- **claude-haiku-5-5** · accelerated-dragon · minor wrong: "after a dubious knight move by White" → The video says the opponent played an unambitious pawn to c3 (18:18-18:27), not a dubious knight move.
- **claude-haiku-5-5** · accelerated-dragon · minor unsupported: "the game became an equal endgame with rooks traded" → He only talks about possibly trading a pair of rooks (24:59, 27:25) and rooks stay on the board later (29:15-29:19, 31:32).
- **claude-haiku-5-5** · extreme-places · minor wrong: "digging tens of thousands of bunkers into mountains and under homes" → The video says nearly 10,000 bunkers are scattered through the country [37:53] and hundreds of thousands are under Swiss homes [38:27], not tens of thousands overall.
- **claude-haiku-5-5** · extreme-places · minor misleading: "and the video ends by noting how the Ukraine war has renewed debate over neutrality and bunkers" → The Ukraine-war debate comes at the end of the Switzerland segment [56:41-1:00:35]; the whole video ends with the Cyprus episode and the Human Element announcement [2:58:27].
- **claude-haiku-5-5** · future-of-capitalism · minor unsupported: "aunque reconoce que estas propuestas aún no tienen una forma clara de aplicarse a escala macroeconómica" → The video voices this criticism only of the ecofeminist proposal ([14:02]-[14:10]), not of degrowth or decrowth communism in general.
- **claude-haiku-5-5** · quantum-ai · minor misleading: "Para mostrar dónde podría ser ventajosa, el autor usa dos ejemplos: separar pacientes sanos y enfermos con una frontera que la IA clásica no puede trazar bien" → The patient example (6:11-9:30) is a classical SVM warm-up solved with a squaring trick, not an example of quantum advantage; only the roulette is the quantum case (11:44-15:29).

## Tokens

- claude-haiku-5-5: 10 calls, 136,475 input + 3,745 output tokens, about $0.016

## Code checks failed

- claude-haiku-5-5 · accelerated-dragon: direct_start
- claude-haiku-5-5 · argentine-anarchism: direct_start
- claude-haiku-5-5 · browser-tabs: direct_start
- claude-haiku-5-5 · cross-entropy: direct_start
- claude-haiku-5-5 · extreme-places: length_3_to_5_sentences
- claude-haiku-5-5 · future-of-capitalism: direct_start
- claude-haiku-5-5 · quantum-ai: direct_start
- claude-haiku-5-5 · stonewall-e4-knight: direct_start
