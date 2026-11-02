# Summary eval: prompt v2, judge claude-sonnet

| model | contradicted | critical stated | facts stated | code checks | avg time |
|---|---|---|---|---|---|
| qwen3:4b | 2/50 | 18/22 | 32/50 | 35/40 | 328s |
| gemma3:4b | 4/50 | 13/22 | 22/50 | 36/40 | 92s |

## Failed critical facts and contradictions

- **qwen3:4b** · accelerated-dragon · `game1-technical-endgame-win` → contradicted: Summary says in the first game he "realized he lost", but the fact says he won it.
- **qwen3:4b** · accelerated-dragon · `game2-advantage-lost-then-won` → contradicted: Summary says the second game "ended in a draw", but the fact says the streamer won it.
- **qwen3:4b** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → missing: Summary never mentions the 1880-1916 conservative order or the oligarchy golden-age narrative.
- **qwen3:4b** · extreme-places · `neom-fallen-apart` → missing: Summary only says Neom is "facing construction challenges", without saying it has largely fallen apart or was meant to replace the oil economy.
- **qwen3:4b** · future-of-capitalism · `decoupling-would-come-too-late` → missing: No mention of absolute decoupling or its timeline.
- **qwen3:4b** · stonewall-e4-knight · `white-vs-black-stonewall-knight` → missing: The summary mentions a query about a knight on e4 in the Stonewall but never says whose Stonewall or whose knight, nor that it is a viewer's tweet about the Dutch with Black's knight.
- **gemma3:4b** · accelerated-dragon · `three-games-as-black` → contradicted: The summary describes a single game ('attempts to play the accelerated dragon opening against an opponent'), not three games.
- **gemma3:4b** · argentine-anarchism · `anarchist-golden-age-early-1900s` → contradicted: The summary places the anarchist golden age in 'entre 1880 y 1916' rather than the early 1900s after FORA's 1901 founding.
- **gemma3:4b** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → contradicted: The summary presents 1880-1916 as anarchism's 'Edad de Oro' instead of a narrative of the oligarchy that the video rejects.
- **gemma3:4b** · build-claude-code · `takeaway-demystify` → missing: Summary only says the project aims to 'desmitificar el uso de la IA' and practice harness engineering, without stating agents are just LLM wrappers, that products sell UI, or that it is for learning not production.
- **gemma3:4b** · claude-duet-review · `topic-claude-duet` → missing: The summary calls Claude Duet 'a new personal agent from Anthropic' but never says it is a phone app synced with Claude or aimed at small business owners; the main claim about its form is not stated.
- **gemma3:4b** · extreme-places · `neom-fallen-apart` → missing: It only calls Neom 'ambitious, yet troubled' and does not say it replaces the oil economy or has largely fallen apart.
- **gemma3:4b** · future-of-capitalism · `decoupling-would-come-too-late` → missing: Decoupling is mentioned only as a technological hope; the summary gives no timeline (about 100 years) or argument that it comes too late.
- **gemma3:4b** · quantum-ai · `advantage-only-on-artificial-problem` → missing: Summary does not mention the roulette example or that the proven advantage is on an artificial problem with no practical application.
- **gemma3:4b** · stonewall-e4-knight · `white-vs-black-stonewall-knight` → missing: The summary mentions 'the E4 knight in the Stonewall structure' but never says whose knight it is or that Black plays the Stonewall.
- **gemma3:4b** · stonewall-e4-knight · `dont-take-on-d5` → contradicted: The summary frames the problem as capturing the knight on D5 being a positional mistake, not White's earlier cxd5 or Bg5 as the real mistakes, so it does not state the fact's claim.

## Judge vs human verdicts: 3/3 agree


## Code checks failed

- gemma3:4b · browser-tabs: direct_start
- qwen3:4b · browser-tabs: direct_start
- gemma3:4b · build-claude-code: direct_start
- qwen3:4b · cross-entropy: direct_start
- qwen3:4b · extreme-places: length_3_to_5_sentences, direct_start
- gemma3:4b · future-of-capitalism: direct_start
- gemma3:4b · quantum-ai: direct_start
- qwen3:4b · stonewall-e4-knight: direct_start
